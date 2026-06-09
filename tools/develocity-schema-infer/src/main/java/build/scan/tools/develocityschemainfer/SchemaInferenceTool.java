package build.scan.tools.develocityschemainfer;

import java.io.IOException;
import java.io.InputStream;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.security.MessageDigest;
import java.security.NoSuchAlgorithmException;
import java.util.ArrayList;
import java.util.Comparator;
import java.util.HashMap;
import java.util.HashSet;
import java.util.HexFormat;
import java.util.List;
import java.util.Map;
import java.util.Set;
import java.util.TreeSet;
import java.util.jar.JarEntry;
import java.util.jar.JarFile;
import org.objectweb.asm.ClassReader;
import org.objectweb.asm.ClassVisitor;
import org.objectweb.asm.FieldVisitor;
import org.objectweb.asm.Opcodes;

public final class SchemaInferenceTool {
    private static final String EVENT_MODEL_PREFIX = "com/gradle/scan/eventmodel/";
    private static final Set<String> COLLECTION_TYPES = setOf("java/util/List", "java/util/Collection", "java/util/Set", "java/lang/Iterable");

    private SchemaInferenceTool() {}

    public static void run(Path jar, Path outputDirectory, String coordinates) throws IOException {
        if (!Files.isRegularFile(jar)) {
            throw new IOException("Jar does not exist: " + jar);
        }
        ScanResult scanResult = readEventModelClasses(jar);
        List<ClassModel> classes = scanResult.classes;
        if (classes.isEmpty()) {
            throw new IOException("No event model classes found under " + EVENT_MODEL_PREFIX + " in " + jar);
        }

        Map<String, ClassModel> byInternalName = new HashMap<>();
        Set<String> enumNames = new TreeSet<>();
        for (ClassModel model : classes) {
            byInternalName.put(model.internalName, model);
            if (model.isEnum()) {
                enumNames.add(model.internalName);
            }
        }

        List<ManifestIssue> issues = new ArrayList<>();
        Files.createDirectories(outputDirectory);
        for (ClassModel model : classes) {
            Path schemaPath = outputDirectory.resolve(model.internalName + ".avsc");
            Files.createDirectories(schemaPath.getParent());
            String schema = model.isEnum()
                    ? enumSchema(model)
                    : recordSchema(model, byInternalName, enumNames, issues);
            Files.writeString(schemaPath, schema, StandardCharsets.UTF_8);
        }
        Files.writeString(outputDirectory.resolve("manifest.json"),
                manifest(coordinates, jar, classes.size(), classes.size(), issues, scanResult.skippedClasses), StandardCharsets.UTF_8);
    }

    private static ScanResult readEventModelClasses(Path jar) throws IOException {
        List<ClassModel> classes = new ArrayList<>();
        List<SkippedClass> skippedClasses = new ArrayList<>();
        try (JarFile jarFile = new JarFile(jar.toFile())) {
            List<JarEntry> entries = new ArrayList<>();
            java.util.Enumeration<JarEntry> allEntries = jarFile.entries();
            while (allEntries.hasMoreElements()) {
                JarEntry entry = allEntries.nextElement();
                if (!entry.isDirectory()
                        && entry.getName().startsWith(EVENT_MODEL_PREFIX)
                        && entry.getName().endsWith(".class")) {
                    if (entry.getName().contains("$")) {
                        skippedClasses.add(new SkippedClass(
                                entry.getName().substring(0, entry.getName().length() - ".class".length()),
                                "inner class is not emitted as a top-level Avro schema"));
                    } else {
                        entries.add(entry);
                    }
                }
            }
            entries.sort(Comparator.comparing(JarEntry::getName));
            skippedClasses.sort(Comparator.comparing((SkippedClass skippedClass) -> skippedClass.className)
                    .thenComparing(skippedClass -> skippedClass.reason));
            for (JarEntry entry : entries) {
                try (InputStream in = jarFile.getInputStream(entry)) {
                    classes.add(readClass(in));
                }
            }
        }
        classes.sort(Comparator.comparing(model -> model.internalName));
        return new ScanResult(classes, skippedClasses);
    }

    private static ClassModel readClass(InputStream input) throws IOException {
        ClassReader reader = new ClassReader(input);
        ClassModelBuilder builder = new ClassModelBuilder();
        reader.accept(builder, ClassReader.SKIP_CODE | ClassReader.SKIP_DEBUG | ClassReader.SKIP_FRAMES);
        return builder.toModel();
    }

    private static String recordSchema(ClassModel model, Map<String, ClassModel> classes, Set<String> enumNames, List<ManifestIssue> issues) {
        JsonWriter json = new JsonWriter();
        json.beginObject()
                .property("type", "record")
                .property("name", simpleName(model.internalName))
                .property("namespace", namespace(model.internalName))
                .name("fields").beginArray();
        boolean first = true;
        for (FieldModel field : model.fields) {
            if (field.isStatic() || field.isSynthetic()) {
                continue;
            }
            if (!first) {
                json.comma();
            }
            first = false;
            Mapping mapping = mapField(model, field, classes, enumNames, issues);
            json.beginObject().property("name", field.name).name("type");
            if (mapping.nullable) {
                json.beginArray().raw("\"null\"").comma().raw(mapping.avroJson).endArray().comma().propertyNull("default");
            } else {
                json.raw(mapping.avroJson);
            }
            json.endObject();
        }
        json.endArray().endObject().newline();
        return json.toString();
    }

    private static String enumSchema(ClassModel model) {
        JsonWriter json = new JsonWriter();
        json.beginObject()
                .property("type", "enum")
                .property("name", simpleName(model.internalName))
                .property("namespace", namespace(model.internalName))
                .name("symbols").beginArray();
        for (int i = 0; i < model.enumSymbols.size(); i++) {
            if (i > 0) {
                json.comma();
            }
            json.value(model.enumSymbols.get(i));
        }
        json.endArray().endObject().newline();
        return json.toString();
    }

    private static Mapping mapField(ClassModel owner, FieldModel field, Map<String, ClassModel> classes, Set<String> enumNames, List<ManifestIssue> issues) {
        ParsedType type = SignatureParser.parse(field.signature != null ? field.signature : field.descriptor);
        return mapType(type, owner, field, classes, enumNames, issues, false);
    }

    private static Mapping mapType(ParsedType type, ClassModel owner, FieldModel field, Map<String, ClassModel> classes, Set<String> enumNames, List<ManifestIssue> issues, boolean nested) {
        switch (type.kind) {
            case BOOLEAN:
                return new Mapping("\"boolean\"", false);
            case INT:
                return new Mapping("\"int\"", false);
            case LONG:
                return new Mapping("\"long\"", false);
            case FLOAT:
                return new Mapping("\"float\"", false);
            case DOUBLE:
                return new Mapping("\"double\"", false);
            case STRING:
                return new Mapping("\"string\"", !nested);
            case BYTES:
                return new Mapping("\"bytes\"", !nested);
            case ARRAY:
                return mapArray(type, owner, field, classes, enumNames, issues, nested);
            case OBJECT:
                return mapObject(type, owner, field, classes, enumNames, issues, nested);
            case UNKNOWN:
            default:
                return ambiguous(owner, field, issues, type.reason == null ? "unsupported or unknown type" : type.reason, nested);
        }
    }

    private static Mapping mapArray(ParsedType type, ClassModel owner, FieldModel field, Map<String, ClassModel> classes, Set<String> enumNames, List<ManifestIssue> issues, boolean nested) {
        Mapping element = mapType(type.arguments.get(0), owner, field, classes, enumNames, issues, true);
        return new Mapping("{\"type\":\"array\",\"items\":" + element.avroJson + "}", !nested);
    }

    private static Mapping mapObject(ParsedType type, ClassModel owner, FieldModel field, Map<String, ClassModel> classes, Set<String> enumNames, List<ManifestIssue> issues, boolean nested) {
        String name = type.internalName;
        if (isStringLike(name)) {
            return new Mapping("\"string\"", !nested);
        }
        if (isIntegerLike(name)) {
            return new Mapping("\"int\"", !nested);
        }
        if ("java/lang/Long".equals(name)) {
            return new Mapping("\"long\"", !nested);
        }
        if ("java/lang/Float".equals(name)) {
            return new Mapping("\"float\"", !nested);
        }
        if ("java/lang/Double".equals(name)) {
            return new Mapping("\"double\"", !nested);
        }
        if ("java/lang/Boolean".equals(name)) {
            return new Mapping("\"boolean\"", !nested);
        }
        if (isCollectionLike(name)) {
            if (type.arguments.isEmpty()) {
                return ambiguous(owner, field, issues, "generic element type is unavailable", nested);
            }
            Mapping element = mapType(type.arguments.get(0), owner, field, classes, enumNames, issues, true);
            return new Mapping("{\"type\":\"array\",\"items\":" + element.avroJson + "}", !nested);
        }
        if (isMapLike(name)) {
            if (type.arguments.size() < 2) {
                return ambiguous(owner, field, issues, "generic map key/value types are unavailable", nested);
            }
            ParsedType key = type.arguments.get(0);
            if (!(key.kind == TypeKind.STRING || (key.kind == TypeKind.OBJECT && isStringLike(key.internalName)))) {
                return ambiguous(owner, field, issues, "map key type is not string-compatible", nested);
            }
            Mapping value = mapType(type.arguments.get(1), owner, field, classes, enumNames, issues, true);
            return new Mapping("{\"type\":\"map\",\"values\":" + value.avroJson + "}", !nested);
        }
        if (enumNames.contains(name) || classes.containsKey(name)) {
            return new Mapping(namedReference(name), !nested);
        }
        if (name.startsWith(EVENT_MODEL_PREFIX)) {
            return ambiguous(owner, field, issues, "event model reference is not emitted as a schema " + dotted(name), nested);
        }
        return ambiguous(owner, field, issues, "unsupported reference type " + dotted(name), nested);
    }

    private static Mapping ambiguous(ClassModel owner, FieldModel field, List<ManifestIssue> issues, String reason, boolean nested) {
        issues.add(new ManifestIssue(owner.internalName, field.name, reason));
        return new Mapping("\"string\"", !nested);
    }

    private static String namedReference(String internalName) {
        return "\"" + escape(dotted(internalName)) + "\"";
    }

    private static String manifest(
            String coordinates,
            Path jar,
            int classesScanned,
            int schemasEmitted,
            List<ManifestIssue> issues,
            List<SkippedClass> skippedClasses) throws IOException {
        issues.sort(Comparator.comparing((ManifestIssue issue) -> issue.className)
                .thenComparing(issue -> issue.fieldName)
                .thenComparing(issue -> issue.reason));
        skippedClasses.sort(Comparator.comparing((SkippedClass skippedClass) -> skippedClass.className)
                .thenComparing(skippedClass -> skippedClass.reason));
        JsonWriter json = new JsonWriter();
        json.beginObject()
                .property("coordinates", coordinates)
                .property("artifactFile", jar.getFileName().toString())
                .property("sha256", sha256(jar))
                .property("classesScanned", classesScanned)
                .property("schemasEmitted", schemasEmitted)
                .name("skippedClasses").beginArray();
        for (int i = 0; i < skippedClasses.size(); i++) {
            if (i > 0) {
                json.comma();
            }
            SkippedClass skippedClass = skippedClasses.get(i);
            json.beginObject()
                    .property("class", dotted(skippedClass.className))
                    .property("reason", skippedClass.reason)
                    .endObject();
        }
        json.endArray()
                .comma()
                .name("ambiguousMappings").beginArray();
        for (int i = 0; i < issues.size(); i++) {
            if (i > 0) {
                json.comma();
            }
            ManifestIssue issue = issues.get(i);
            json.beginObject()
                    .property("class", dotted(issue.className))
                    .property("field", issue.fieldName)
                    .property("reason", issue.reason)
                    .endObject();
        }
        json.endArray().endObject().newline();
        return json.toString();
    }

    private static String sha256(Path path) throws IOException {
        try {
            MessageDigest digest = MessageDigest.getInstance("SHA-256");
            try (InputStream in = Files.newInputStream(path)) {
                byte[] buffer = new byte[8192];
                int read;
                while ((read = in.read(buffer)) != -1) {
                    digest.update(buffer, 0, read);
                }
            }
            return HexFormat.of().formatHex(digest.digest());
        } catch (NoSuchAlgorithmException e) {
            throw new IllegalStateException(e);
        }
    }

    private static boolean isStringLike(String name) {
        return "java/lang/String".equals(name) || "java/lang/CharSequence".equals(name) || "java/lang/Character".equals(name);
    }

    private static boolean isIntegerLike(String name) {
        return "java/lang/Byte".equals(name) || "java/lang/Short".equals(name) || "java/lang/Integer".equals(name);
    }

    private static boolean isCollectionLike(String name) {
        return COLLECTION_TYPES.contains(name);
    }

    private static boolean isMapLike(String name) {
        return "java/util/Map".equals(name);
    }

    private static String simpleName(String internalName) {
        int slash = internalName.lastIndexOf('/');
        return slash >= 0 ? internalName.substring(slash + 1) : internalName;
    }

    private static String namespace(String internalName) {
        int slash = internalName.lastIndexOf('/');
        return slash < 0 ? "" : internalName.substring(0, slash).replace('/', '.');
    }

    private static String dotted(String internalName) {
        return internalName.replace('/', '.');
    }

    private static String escape(String value) {
        StringBuilder out = new StringBuilder(value.length() + 8);
        for (int i = 0; i < value.length(); i++) {
            char c = value.charAt(i);
            switch (c) {
                case '\\':
                    out.append("\\\\");
                    break;
                case '"':
                    out.append("\\\"");
                    break;
                case '\n':
                    out.append("\\n");
                    break;
                case '\r':
                    out.append("\\r");
                    break;
                case '\t':
                    out.append("\\t");
                    break;
                default:
                    out.append(c);
            }
        }
        return out.toString();
    }

    private static Set<String> setOf(String... values) {
        Set<String> set = new HashSet<>();
        for (String value : values) {
            set.add(value);
        }
        return set;
    }

    private static final class ClassModelBuilder extends ClassVisitor {
        private String internalName;
        private String superName;
        private int access;
        private final List<FieldModel> fields = new ArrayList<>();
        private final List<String> enumSymbols = new ArrayList<>();

        private ClassModelBuilder() {
            super(Opcodes.ASM9);
        }

        @Override
        public void visit(int version, int access, String name, String signature, String superName, String[] interfaces) {
            this.internalName = name;
            this.superName = superName;
            this.access = access;
        }

        @Override
        public FieldVisitor visitField(int access, String name, String descriptor, String signature, Object value) {
            FieldModel field = new FieldModel(access, name, descriptor, signature);
            fields.add(field);
            if ((access & Opcodes.ACC_ENUM) != 0) {
                enumSymbols.add(name);
            }
            return null;
        }

        ClassModel toModel() {
            return new ClassModel(internalName, superName, access, new ArrayList<>(fields), new ArrayList<>(enumSymbols));
        }
    }

    private static final class ScanResult {
        final List<ClassModel> classes;
        final List<SkippedClass> skippedClasses;

        ScanResult(List<ClassModel> classes, List<SkippedClass> skippedClasses) {
            this.classes = classes;
            this.skippedClasses = skippedClasses;
        }
    }

    private static final class ClassModel {
        final String internalName;
        final String superName;
        final int access;
        final List<FieldModel> fields;
        final List<String> enumSymbols;

        ClassModel(String internalName, String superName, int access, List<FieldModel> fields, List<String> enumSymbols) {
            this.internalName = internalName;
            this.superName = superName;
            this.access = access;
            this.fields = fields;
            this.enumSymbols = enumSymbols;
        }

        boolean isEnum() {
            return (access & Opcodes.ACC_ENUM) != 0 || "java/lang/Enum".equals(superName);
        }
    }

    private static final class FieldModel {
        final int access;
        final String name;
        final String descriptor;
        final String signature;

        FieldModel(int access, String name, String descriptor, String signature) {
            this.access = access;
            this.name = name;
            this.descriptor = descriptor;
            this.signature = signature;
        }

        boolean isStatic() {
            return (access & Opcodes.ACC_STATIC) != 0;
        }

        boolean isSynthetic() {
            return (access & Opcodes.ACC_SYNTHETIC) != 0;
        }
    }

    private static final class Mapping {
        final String avroJson;
        final boolean nullable;

        Mapping(String avroJson, boolean nullable) {
            this.avroJson = avroJson;
            this.nullable = nullable;
        }
    }

    private static final class SkippedClass {
        final String className;
        final String reason;

        SkippedClass(String className, String reason) {
            this.className = className;
            this.reason = reason;
        }
    }

    private static final class ManifestIssue {
        final String className;
        final String fieldName;
        final String reason;

        ManifestIssue(String className, String fieldName, String reason) {
            this.className = className;
            this.fieldName = fieldName;
            this.reason = reason;
        }
    }

    private enum TypeKind { BOOLEAN, INT, LONG, FLOAT, DOUBLE, STRING, BYTES, ARRAY, OBJECT, UNKNOWN }

    private static final class ParsedType {
        final TypeKind kind;
        final String internalName;
        final List<ParsedType> arguments;
        final String reason;

        private ParsedType(TypeKind kind, String internalName, List<ParsedType> arguments, String reason) {
            this.kind = kind;
            this.internalName = internalName;
            this.arguments = arguments;
            this.reason = reason;
        }

        static ParsedType primitive(TypeKind kind) {
            return new ParsedType(kind, null, List.of(), null);
        }

        static ParsedType object(String internalName, List<ParsedType> arguments) {
            return new ParsedType(TypeKind.OBJECT, internalName, new ArrayList<>(arguments), null);
        }

        static ParsedType array(ParsedType element) {
            List<ParsedType> arguments = new ArrayList<>();
            arguments.add(element);
            return new ParsedType(TypeKind.ARRAY, null, arguments, null);
        }

        static ParsedType unknown(String reason) {
            return new ParsedType(TypeKind.UNKNOWN, null, List.of(), reason);
        }
    }

    private static final class SignatureParser {
        static ParsedType parse(String signatureOrDescriptor) {
            Index index = new Index();
            return parseType(signatureOrDescriptor, index);
        }

        private static ParsedType parseType(String text, Index index) {
            if (index.value >= text.length()) {
                return ParsedType.unknown("empty type descriptor");
            }
            char c = text.charAt(index.value++);
            switch (c) {
                case 'Z':
                    return ParsedType.primitive(TypeKind.BOOLEAN);
                case 'B':
                case 'S':
                case 'I':
                    return ParsedType.primitive(TypeKind.INT);
                case 'J':
                    return ParsedType.primitive(TypeKind.LONG);
                case 'F':
                    return ParsedType.primitive(TypeKind.FLOAT);
                case 'D':
                    return ParsedType.primitive(TypeKind.DOUBLE);
                case 'C':
                    return ParsedType.primitive(TypeKind.STRING);
                case '[':
                    return parseArray(text, index);
                case 'L':
                    return parseObject(text, index);
                case 'T':
                    return parseTypeVariable(text, index);
                default:
                    return ParsedType.unknown("unsupported descriptor token '" + c + "'");
            }
        }

        private static ParsedType parseArray(String text, Index index) {
            if (index.value < text.length() && text.charAt(index.value) == 'B') {
                index.value++;
                return ParsedType.primitive(TypeKind.BYTES);
            }
            return ParsedType.array(parseType(text, index));
        }

        private static ParsedType parseObject(String text, Index index) {
            StringBuilder name = new StringBuilder();
            List<ParsedType> arguments = new ArrayList<>();
            while (index.value < text.length()) {
                char c = text.charAt(index.value++);
                if (c == ';') {
                    return ParsedType.object(name.toString(), arguments);
                }
                if (c == '<') {
                    while (index.value < text.length() && text.charAt(index.value) != '>') {
                        char variance = text.charAt(index.value);
                        if (variance == '+' || variance == '-') {
                            index.value++;
                        } else if (variance == '*') {
                            index.value++;
                            arguments.add(ParsedType.unknown("wildcard generic type is unavailable"));
                            continue;
                        }
                        arguments.add(parseType(text, index));
                    }
                    if (index.value < text.length() && text.charAt(index.value) == '>') {
                        index.value++;
                    }
                } else {
                    name.append(c);
                }
            }
            return ParsedType.unknown("unterminated object descriptor");
        }

        private static ParsedType parseTypeVariable(String text, Index index) {
            while (index.value < text.length() && text.charAt(index.value++) != ';') {
                // Skip type variable name.
            }
            return ParsedType.unknown("generic type variable is unavailable");
        }

        private static final class Index {
            int value;
        }
    }

    private static final class JsonWriter {
        private final StringBuilder out = new StringBuilder();

        JsonWriter beginObject() { out.append('{'); return this; }
        JsonWriter endObject() { out.append('}'); return this; }
        JsonWriter beginArray() { out.append('['); return this; }
        JsonWriter endArray() { out.append(']'); return this; }
        JsonWriter comma() { out.append(','); return this; }
        JsonWriter name(String name) { value(name); out.append(": "); return this; }
        JsonWriter property(String name, String value) { name(name).value(value).comma(); return this; }
        JsonWriter property(String name, int value) { name(name).raw(Integer.toString(value)).comma(); return this; }
        JsonWriter propertyNull(String name) { name(name).raw("null"); return this; }
        JsonWriter value(String value) { out.append('"').append(escape(value)).append('"'); return this; }
        JsonWriter raw(String raw) { out.append(raw); return this; }
        JsonWriter newline() { out.append('\n'); return this; }

        @Override
        public String toString() {
            return out.toString().replace(",}", "}").replace(",]", "]");
        }
    }
}
