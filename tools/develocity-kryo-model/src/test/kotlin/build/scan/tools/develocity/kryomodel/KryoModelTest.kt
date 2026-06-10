package build.scan.tools.develocity.kryomodel

import arrow.core.raise.either
import build.scan.tools.develocity.kryomodel.generated.v4_4_2.Develocity442WireModel
import java.nio.file.Files
import java.nio.file.Path
import kotlin.io.path.name
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertIs
import kotlin.test.assertTrue
import kotlinx.coroutines.runBlocking
import org.objectweb.asm.ClassWriter
import org.objectweb.asm.Handle
import org.objectweb.asm.Label
import org.objectweb.asm.Opcodes
import org.objectweb.asm.Type

class KryoModelTest {
    @Test
    fun metadataAndConfidenceFilteringUseImmutableDomainObjects() {
        val confident = DiscoveredClass(
            internalName = "com/gradle/scan/eventmodel/gradle/TaskStarted_1_0",
            binaryName = "com.gradle.scan.eventmodel.gradle.TaskStarted_1_0",
            simpleName = "TaskStarted_1_0",
            fields = listOf(WireField("path", "Ljava/lang/String;", "java.lang.String")),
            evidence = listOf(
                DiscoveryEvidence(
                    EvidenceKind.RegistryReferencesEventModel,
                    "com/gradle/scan/agent/serialization/scan/serializer/kryo/a",
                    "registered serializer evidence",
                ),
            ),
            confidence = Confidence.High,
        )
        val ambiguous = confident.copy(internalName = "com/gradle/scan/eventmodel/gradle/Ambiguous_1_0", confidence = Confidence.Medium)

        assertEquals(listOf(confident), filterConfident(listOf(ambiguous, confident)))
    }

    @Test
    fun highConfidenceRequiresKryoSerializerEventReferenceAndRegistryReference() {
        val pluginVersion = PluginVersion("4.4.2")
        val eventClass = parsedClass("com/gradle/scan/eventmodel/gradle/TaskStarted_1_0")
        val highSerializer = parsedClass(
            "com/gradle/scan/agent/serialization/scan/serializer/kryo/RegisteredSerializer",
            references = setOf(eventClass.internalName),
        )
        val mediumSerializer = parsedClass(
            "com/gradle/scan/agent/serialization/scan/serializer/kryo/UnregisteredSerializer",
            references = setOf(eventClass.internalName),
        )
        val registry = parsedClass(
            "com/gradle/scan/agent/serialization/scan/serializer/kryo/KryoEventDataSerializerRegistryGradle",
            references = setOf(highSerializer.internalName),
        )

        val report = discoverKryoWireModel(pluginVersion, listOf(eventClass, highSerializer, mediumSerializer, registry))

        assertEquals(Confidence.High, report.discoveredClasses.single().confidence)
        assertTrue(report.discoveredClasses.single().evidence.any { it.kind == EvidenceKind.KryoSerializerPackage })
        assertTrue(report.discoveredClasses.single().evidence.any { it.kind == EvidenceKind.RegistryReferencesEventModel })
    }

    @Test
    fun discoveryPreservesAsmFieldDeclarationOrder() {
        val eventClass = parsedClass(
            "com/gradle/scan/eventmodel/gradle/TaskStarted_1_0",
            fields = listOf(
                WireField("zLast", "Ljava/lang/String;", "java.lang.String"),
                WireField("aFirstAlphabetically", "I", "Int"),
            ),
        )
        val serializer = parsedClass(
            "com/gradle/scan/agent/serialization/scan/serializer/kryo/RegisteredSerializer",
            references = setOf(eventClass.internalName),
        )
        val registry = parsedClass(
            "com/gradle/scan/agent/serialization/scan/serializer/kryo/KryoEventDataSerializerRegistryGradle",
            references = setOf(serializer.internalName),
        )

        val report = discoverKryoWireModel(PluginVersion("4.4.2"), listOf(eventClass, serializer, registry))

        assertEquals(listOf("zLast", "aFirstAlphabetically"), report.wireClasses.single().fields.map { it.name })
    }

    @Test
    fun sourceWriteFailuresAreRaised() = runBlocking {
        val fileAsRoot = Files.createTempFile("kryo-model-output", ".kt")
        val result = either {
            JdkGenerationContext(fileAsRoot).writeSource(generatedRelativePath(), "content")
        }

        assertIs<GenerationError.SourceWriteFailed>(result.leftOrNull())
        Unit
    }

    @Test
    fun rendererRaisesInvalidClassNames() {
        val result = either {
            renderKotlinSources(
                listOf(
                    WireClass(
                        pluginVersion = PluginVersion("4.4.2"),
                        internalName = "",
                        binaryName = "",
                        simpleName = "",
                        fields = emptyList(),
                        serializerBindings = emptyList(),
                        evidence = emptyList(),
                        confidence = Confidence.High,
                    ),
                ),
            )
        }

        assertEquals(GenerationError.InvalidClassName(""), result.leftOrNull())
    }

    @Test
    fun asmScannerCollectsReferencesFromSignaturesAnnotationsInvokeDynamicAndRecordComponents() = runBlocking {
        val parsed = either {
            AsmBytecodeScanContext().parseClass(JarClassEntry("Fixture.class", asmReferenceFixture()))
        }.getOrNull()!!

        val expected = setOf(
            "com/gradle/scan/eventmodel/gradle/ClassSignature_1_0",
            "com/gradle/scan/eventmodel/gradle/FieldSignature_1_0",
            "com/gradle/scan/eventmodel/gradle/FieldAnnotation_1_0",
            "com/gradle/scan/eventmodel/gradle/MethodSignature_1_0",
            "com/gradle/scan/eventmodel/gradle/MethodAnnotation_1_0",
            "com/gradle/scan/eventmodel/gradle/InvokeDynamicDescriptor_1_0",
            "com/gradle/scan/eventmodel/gradle/BootstrapOwner_1_0",
            "com/gradle/scan/eventmodel/gradle/BootstrapArgument_1_0",
            "com/gradle/scan/eventmodel/gradle/LocalVariableSignature_1_0",
            "com/gradle/scan/eventmodel/gradle/RecordComponent_1_0",
            "com/gradle/scan/eventmodel/gradle/RecordComponentSignature_1_0",
            "com/gradle/scan/eventmodel/gradle/RecordComponentAnnotation_1_0",
        )
        assertTrue(parsed.referencedInternalNames.containsAll(expected), parsed.referencedInternalNames.toString())
    }

    @Test
    fun arrowRaiseReportsEmptyJarInput() = runBlocking {
        val missing = Path.of("/tmp/definitely-missing-develocity.jar")
        val result = either {
            with(JdkJarReadContext()) {
                with(AsmBytecodeScanContext()) {
                    scanDevelocityPluginJar(PluginVersion("4.4.2"), missing)
                }
            }
        }

        assertIs<DiscoveryError.JarReadFailed>(result.leftOrNull())
        Unit
    }

    @Test
    fun renderingIsDeterministicAndNeutral() {
        val source = either {
            renderKotlinSources(
                listOf(
                    WireClass(
                        pluginVersion = PluginVersion("4.4.2"),
                        internalName = "com/gradle/scan/eventmodel/gradle/TaskStarted_1_0",
                        binaryName = "com.gradle.scan.eventmodel.gradle.TaskStarted_1_0",
                        simpleName = "TaskStarted_1_0",
                        fields = listOf(WireField("path", "Ljava/lang/String;", "java.lang.String")),
                        serializerBindings = listOf(
                            KryoSerializerBinding(
                                wireClassInternalName = "com/gradle/scan/eventmodel/gradle/TaskStarted_1_0",
                                serializerInternalName = "com/gradle/scan/agent/serialization/scan/serializer/kryo/jc",
                                evidence = emptyList(),
                                confidence = Confidence.High,
                            ),
                        ),
                        evidence = emptyList(),
                        confidence = Confidence.High,
                    ),
                ),
            )
        }.getOrNull()!![generatedRelativePath()]!!

        assertTrue("GeneratedWireClass" in source)
        assertTrue("TaskStarted_1_0" in source)
        assertTrue("does not contain decompiled Develocity implementations" in source)
    }

    @Test
    fun develocity442ScanMatchesCommittedGeneratedModel() = runBlocking {
        val jar = locateDevelocityPluginJar()
        val report = either {
            with(JdkJarReadContext()) {
                with(AsmBytecodeScanContext()) {
                    scanDevelocityPluginJar(PluginVersion("4.4.2"), jar)
                }
            }
        }.getOrNull()!!
        val generated = either { renderKotlinSources(report.wireClasses) }.getOrNull()!![generatedRelativePath()]!!
        val committed = Files.readString(locateCommittedGeneratedSource(generatedRelativePath()))

        assertTrue(report.wireClasses.isNotEmpty(), "expected confident Kryo wire classes from Develocity 4.4.2")
        assertEquals(committed, generated)
        assertEquals(report.wireClasses.size, Develocity442WireModel.classes.size)
    }
}

private fun parsedClass(
    internalName: String,
    references: Set<String> = emptySet(),
    fields: List<WireField> = emptyList(),
): ParsedClass = ParsedClass(
    internalName = internalName,
    superName = "java/lang/Object",
    interfaces = emptyList(),
    fields = fields,
    referencedInternalNames = references,
)

private fun asmReferenceFixture(): ByteArray {
    val writer = ClassWriter(0)
    writer.visit(
        Opcodes.V17,
        Opcodes.ACC_PUBLIC or Opcodes.ACC_SUPER or Opcodes.ACC_RECORD,
        "fixture/ReferenceFixture",
        "Lcom/gradle/scan/eventmodel/gradle/ClassSignature_1_0;",
        "java/lang/Record",
        null,
    )
    writer.visitAnnotation("Lcom/gradle/scan/eventmodel/gradle/ClassAnnotation_1_0;", true)?.visitEnd()
    writer.visitField(
        Opcodes.ACC_PRIVATE,
        "field",
        "Ljava/lang/String;",
        "Lcom/gradle/scan/eventmodel/gradle/FieldSignature_1_0;",
        null,
    )?.apply {
        visitAnnotation("Lcom/gradle/scan/eventmodel/gradle/FieldAnnotation_1_0;", true)?.visitEnd()
        visitEnd()
    }
    writer.visitRecordComponent(
        "component",
        "Lcom/gradle/scan/eventmodel/gradle/RecordComponent_1_0;",
        "Lcom/gradle/scan/eventmodel/gradle/RecordComponentSignature_1_0;",
    )?.apply {
        visitAnnotation("Lcom/gradle/scan/eventmodel/gradle/RecordComponentAnnotation_1_0;", true)?.visitEnd()
        visitEnd()
    }
    writer.visitMethod(
        Opcodes.ACC_PUBLIC,
        "method",
        "()V",
        "(Lcom/gradle/scan/eventmodel/gradle/MethodSignature_1_0;)V",
        null,
    )?.apply {
        visitAnnotation("Lcom/gradle/scan/eventmodel/gradle/MethodAnnotation_1_0;", true)?.visitEnd()
        visitCode()
        val start = Label()
        val end = Label()
        visitLabel(start)
        visitInvokeDynamicInsn(
            "run",
            "(Lcom/gradle/scan/eventmodel/gradle/InvokeDynamicDescriptor_1_0;)V",
            Handle(
                Opcodes.H_INVOKESTATIC,
                "com/gradle/scan/eventmodel/gradle/BootstrapOwner_1_0",
                "bootstrap",
                "(Ljava/lang/invoke/MethodHandles\$Lookup;Ljava/lang/String;Ljava/lang/invoke/MethodType;)Ljava/lang/Object;",
                false,
            ),
            Type.getType("Lcom/gradle/scan/eventmodel/gradle/BootstrapArgument_1_0;"),
        )
        visitLocalVariable(
            "local",
            "Ljava/lang/Object;",
            "Lcom/gradle/scan/eventmodel/gradle/LocalVariableSignature_1_0;",
            start,
            end,
            0,
        )
        visitLabel(end)
        visitInsn(Opcodes.RETURN)
        visitMaxs(0, 1)
        visitEnd()
    }
    writer.visitEnd()
    return writer.toByteArray()
}

private fun locateCommittedGeneratedSource(relativePath: String): Path {
    val runfiles = System.getenv("TEST_SRCDIR")?.let { Path.of(it) }
    if (runfiles != null) {
        Files.walk(runfiles).use { paths ->
            val source = paths.filter { it.endsWith("tools/develocity-kryo-model/src/generated/kotlin/$relativePath") }
                .findFirst()
            if (source.isPresent) return source.get()
        }
    }
    return Path.of(System.getProperty("user.dir")).resolve("tools/develocity-kryo-model/src/generated/kotlin").resolve(relativePath)
}

private fun locateDevelocityPluginJar(): Path {
    val classpathMatch = System.getProperty("java.class.path")
        .split(System.getProperty("path.separator"))
        .map { Path.of(it) }
        .firstOrNull { it.name.contains("develocity-gradle-plugin") && it.name.endsWith(".jar") }
    if (classpathMatch != null) return classpathMatch

    val runfiles = System.getenv("TEST_SRCDIR")?.let { Path.of(it) }
        ?: error("TEST_SRCDIR is not set and develocity jar is not on the classpath")
    return Files.walk(runfiles).use { paths ->
        paths.filter { it.name.contains("develocity-gradle-plugin") && it.name.endsWith(".jar") }
            .findFirst()
            .orElseThrow { IllegalStateException("Could not locate develocity-gradle-plugin jar in runfiles") }
    }
}
