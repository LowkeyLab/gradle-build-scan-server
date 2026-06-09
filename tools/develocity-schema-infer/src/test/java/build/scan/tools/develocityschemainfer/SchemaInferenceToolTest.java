package build.scan.tools.develocityschemainfer;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.io.IOException;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.Comparator;
import java.util.List;
import java.util.Map;
import java.util.jar.JarEntry;
import java.util.jar.JarOutputStream;
import org.apache.avro.Schema;
import org.junit.jupiter.api.Test;
import org.objectweb.asm.ClassWriter;
import org.objectweb.asm.FieldVisitor;
import org.objectweb.asm.Opcodes;

final class SchemaInferenceToolTest {
    @Test
    void infersDeterministicSchemasAndManifestFromEventModelBytecode() throws Exception {
        Path tempDir = Files.createTempDirectory("schema-infer-test");
        try {
            Path jar = tempDir.resolve("fixture.jar");
            writeFixtureJar(jar);

            Path output = tempDir.resolve("schemas");
            SchemaInferenceTool.run(jar, output, "fixture:fixture:1.0");
            Map<String, String> firstRun = readAllOutput(output);
            SchemaInferenceTool.run(jar, output, "fixture:fixture:1.0");
            Map<String, String> secondRun = readAllOutput(output);

            assertEquals(firstRun, secondRun, "generation should be stable when rerun");

            String sampleJson = Files.readString(output.resolve("com/gradle/scan/eventmodel/Sample.avsc"));
            assertTrue(sampleJson.contains("\"type\": [\"null\",\"com.gradle.scan.eventmodel.Child\"]"));
            assertTrue(sampleJson.contains("\"items\":\"com.gradle.scan.eventmodel.Child\""));
            assertTrue(sampleJson.contains("\"values\":\"com.gradle.scan.eventmodel.Child\""));
            assertTrue(sampleJson.contains("\"name\": \"byId\",\"type\": [\"null\",\"string\"],\"default\": null}"));
            assertFalse(sampleJson.contains("\"name\": \"byId\",\"type\": [\"null\",{\"type\":\"map\""));
            assertTrue(sampleJson.contains("\"type\": [\"null\",\"com.gradle.scan.eventmodel.State\"]"));
            assertFalse(sampleJson.contains("\"type\":\"record\""), "event model references should use named types, not inline records");
            assertFalse(sampleJson.contains("\"fields\":[]"), "event model references should not be empty placeholder records");

            Schema.Parser parser = new Schema.Parser();
            parser.parse(Files.readString(output.resolve("com/gradle/scan/eventmodel/Child.avsc")));
            parser.parse(Files.readString(output.resolve("com/gradle/scan/eventmodel/State.avsc")));
            Schema sample = parser.parse(sampleJson);
            assertEquals("record", sample.getType().getName());
            assertEquals("com.gradle.scan.eventmodel", sample.getNamespace());
            assertEquals(List.of(
                    "active", "count", "name", "payload", "child", "children", "byName", "byId", "state", "rawList"),
                    sample.getFields().stream().map(Schema.Field::name).collect(java.util.stream.Collectors.toList()));
            assertEquals(Schema.Type.BOOLEAN, sample.getField("active").schema().getType());
            assertEquals(Schema.Type.INT, sample.getField("count").schema().getType());
            assertEquals(Schema.Type.STRING, nonNullBranch(sample.getField("name").schema()).getType());
            assertEquals(Schema.Type.BYTES, nonNullBranch(sample.getField("payload").schema()).getType());
            assertEquals("Child", nonNullBranch(sample.getField("child").schema()).getName());
            assertEquals(Schema.Type.ARRAY, nonNullBranch(sample.getField("children").schema()).getType());
            assertEquals("Child", nonNullBranch(sample.getField("children").schema()).getElementType().getName());
            assertEquals(Schema.Type.MAP, nonNullBranch(sample.getField("byName").schema()).getType());
            assertEquals("Child", nonNullBranch(sample.getField("byName").schema()).getValueType().getName());
            assertEquals(Schema.Type.STRING, nonNullBranch(sample.getField("byId").schema()).getType());
            assertEquals("State", nonNullBranch(sample.getField("state").schema()).getName());
            assertEquals(Schema.Type.STRING, nonNullBranch(sample.getField("rawList").schema()).getType());
            assertEquals(Schema.Type.NULL, sample.getField("child").schema().getTypes().get(0).getType());
            assertTrue(sampleJson.contains("\"default\": null"));

            Schema enumSchema = new Schema.Parser().parse(Files.readString(output.resolve("com/gradle/scan/eventmodel/State.avsc")));
            assertEquals(Schema.Type.ENUM, enumSchema.getType());
            assertEquals(List.of("STARTED", "FINISHED"), enumSchema.getEnumSymbols());

            String manifest = Files.readString(output.resolve("manifest.json"));
            assertTrue(manifest.contains("\"coordinates\": \"fixture:fixture:1.0\""));
            assertTrue(manifest.contains("\"artifactFile\": \"fixture.jar\""));
            assertFalse(manifest.contains(tempDir.toString()), "manifest should not record absolute local paths");
            assertTrue(manifest.contains("\"classesScanned\": 3"));
            assertTrue(manifest.contains("\"schemasEmitted\": 3"));
            assertTrue(manifest.contains("\"skippedClasses\": [{\"class\": \"com.gradle.scan.eventmodel.Outer$Inner\""));
            assertTrue(manifest.contains("inner class is not emitted as a top-level Avro schema"));
            assertTrue(manifest.contains("rawList"));
            assertTrue(manifest.contains("generic element type is unavailable"));
            assertTrue(manifest.contains("byId"));
            assertTrue(manifest.contains("map key type is not string-compatible"));
            assertFalse(manifest.contains("NotEvent"), "non-eventmodel classes should be ignored");
        } finally {
            deleteRecursively(tempDir);
        }
    }

    private static Schema nonNullBranch(Schema schema) {
        assertEquals(Schema.Type.UNION, schema.getType());
        return schema.getTypes().stream()
                .filter(branch -> branch.getType() != Schema.Type.NULL)
                .findFirst()
                .orElseThrow();
    }

    private static Map<String, String> readAllOutput(Path output) throws IOException {
        try (var paths = Files.walk(output)) {
            return paths.filter(Files::isRegularFile)
                    .sorted()
                    .collect(java.util.stream.Collectors.toMap(
                            path -> output.relativize(path).toString(),
                            path -> {
                                try {
                                    return Files.readString(path);
                                } catch (IOException e) {
                                    throw new RuntimeException(e);
                                }
                            },
                            (a, b) -> a,
                            java.util.LinkedHashMap::new));
        }
    }

    private static void writeFixtureJar(Path jar) throws IOException {
        try (JarOutputStream out = new JarOutputStream(Files.newOutputStream(jar))) {
            addClass(out, sampleClass());
            addClass(out, childClass());
            addClass(out, stateEnum());
            addClass(out, innerEventClass());
            addClass(out, notEventClass());
        }
    }

    private static void addClass(JarOutputStream out, byte[] bytes) throws IOException {
        org.objectweb.asm.ClassReader reader = new org.objectweb.asm.ClassReader(bytes);
        out.putNextEntry(new JarEntry(reader.getClassName() + ".class"));
        out.write(bytes);
        out.closeEntry();
    }

    private static byte[] sampleClass() {
        ClassWriter writer = classWriter("com/gradle/scan/eventmodel/Sample", "java/lang/Object");
        field(writer, "active", "Z", null);
        field(writer, "count", "I", null);
        field(writer, "name", "Ljava/lang/String;", null);
        field(writer, "payload", "[B", null);
        field(writer, "child", "Lcom/gradle/scan/eventmodel/Child;", null);
        field(writer, "children", "Ljava/util/List;", "Ljava/util/List<Lcom/gradle/scan/eventmodel/Child;>;");
        field(writer, "byName", "Ljava/util/Map;", "Ljava/util/Map<Ljava/lang/String;Lcom/gradle/scan/eventmodel/Child;>;");
        field(writer, "byId", "Ljava/util/Map;", "Ljava/util/Map<Ljava/lang/Integer;Lcom/gradle/scan/eventmodel/Child;>;");
        field(writer, "state", "Lcom/gradle/scan/eventmodel/State;", null);
        field(writer, "rawList", "Ljava/util/List;", null);
        writer.visitEnd();
        return writer.toByteArray();
    }

    private static byte[] childClass() {
        ClassWriter writer = classWriter("com/gradle/scan/eventmodel/Child", "java/lang/Object");
        field(writer, "id", "J", null);
        writer.visitEnd();
        return writer.toByteArray();
    }

    private static byte[] stateEnum() {
        ClassWriter writer = new ClassWriter(0);
        writer.visit(Opcodes.V17, Opcodes.ACC_PUBLIC | Opcodes.ACC_FINAL | Opcodes.ACC_SUPER | Opcodes.ACC_ENUM,
                "com/gradle/scan/eventmodel/State", null, "java/lang/Enum", null);
        field(writer, "STARTED", "Lcom/gradle/scan/eventmodel/State;", null, Opcodes.ACC_PUBLIC | Opcodes.ACC_STATIC | Opcodes.ACC_FINAL | Opcodes.ACC_ENUM);
        field(writer, "FINISHED", "Lcom/gradle/scan/eventmodel/State;", null, Opcodes.ACC_PUBLIC | Opcodes.ACC_STATIC | Opcodes.ACC_FINAL | Opcodes.ACC_ENUM);
        writer.visitEnd();
        return writer.toByteArray();
    }

    private static byte[] innerEventClass() {
        ClassWriter writer = classWriter("com/gradle/scan/eventmodel/Outer$Inner", "java/lang/Object");
        field(writer, "skipped", "I", null);
        writer.visitEnd();
        return writer.toByteArray();
    }

    private static byte[] notEventClass() {
        ClassWriter writer = classWriter("example/NotEvent", "java/lang/Object");
        field(writer, "ignored", "I", null);
        writer.visitEnd();
        return writer.toByteArray();
    }

    private static ClassWriter classWriter(String name, String superName) {
        ClassWriter writer = new ClassWriter(0);
        writer.visit(Opcodes.V17, Opcodes.ACC_PUBLIC | Opcodes.ACC_SUPER, name, null, superName, null);
        return writer;
    }

    private static void field(ClassWriter writer, String name, String descriptor, String signature) {
        field(writer, name, descriptor, signature, Opcodes.ACC_PUBLIC);
    }

    private static void field(ClassWriter writer, String name, String descriptor, String signature, int access) {
        FieldVisitor visitor = writer.visitField(access, name, descriptor, signature, null);
        visitor.visitEnd();
    }

    private static void deleteRecursively(Path root) throws IOException {
        if (!Files.exists(root)) {
            return;
        }
        try (var paths = Files.walk(root)) {
            for (Path path : paths.sorted(Comparator.reverseOrder()).toList()) {
                Files.delete(path);
            }
        }
    }
}
