package build.scan.tools.develocity.kryomodel

import arrow.core.raise.Raise
import java.nio.file.Files
import java.nio.file.Path
import java.util.jar.JarInputStream
import org.objectweb.asm.AnnotationVisitor
import org.objectweb.asm.ClassReader
import org.objectweb.asm.ClassVisitor
import org.objectweb.asm.FieldVisitor
import org.objectweb.asm.Handle
import org.objectweb.asm.MethodVisitor
import org.objectweb.asm.Opcodes
import org.objectweb.asm.RecordComponentVisitor
import org.objectweb.asm.Type
import org.objectweb.asm.signature.SignatureReader
import org.objectweb.asm.signature.SignatureVisitor

class JdkJarReadContext : JarReadContext {
    context(raise: Raise<DiscoveryError>)
    override suspend fun readClassEntries(jar: Path): List<JarClassEntry> =
        runCatching {
            Files.newInputStream(jar).use { input ->
                JarInputStream(input).use { jarInput ->
                    generateSequence { jarInput.nextJarEntry }
                        .filter { !it.isDirectory && it.name.endsWith(".class") }
                        .map { entry -> JarClassEntry(entry.name, jarInput.readBytes()) }
                        .toList()
                }
            }
        }.getOrElse { error ->
            raise.raise(DiscoveryError.JarReadFailed(jar.toString(), error.message ?: error.javaClass.name))
        }
}

class AsmBytecodeScanContext : BytecodeScanContext {
    context(raise: Raise<DiscoveryError>)
    override suspend fun parseClass(entry: JarClassEntry): ParsedClass =
        runCatching { parseClassUnchecked(entry.bytes) }.getOrElse { error ->
            raise.raise(DiscoveryError.ClassReadFailed(entry.name, error.message ?: error.javaClass.name))
        }
}

class JdkGenerationContext(
    private val outputRoot: Path,
    private val jarReader: JarReadContext = JdkJarReadContext(),
    private val bytecodeScanner: BytecodeScanContext = AsmBytecodeScanContext(),
) : GenerationContext,
    JarReadContext by jarReader,
    BytecodeScanContext by bytecodeScanner {
    context(raise: Raise<GenerationError>)
    override suspend fun writeSource(relativePath: String, content: String) {
        runCatching {
            val output = outputRoot.resolve(relativePath)
            Files.createDirectories(output.parent)
            Files.writeString(output, content)
        }.getOrElse { error ->
            raise.raise(GenerationError.SourceWriteFailed(relativePath, error.message ?: error.javaClass.name))
        }
    }
}

private fun parseClassUnchecked(bytes: ByteArray): ParsedClass {
    val reader = ClassReader(bytes)
    val references = linkedSetOf<String>()
    val fields = mutableListOf<WireField>()
    val visitor = object : ClassVisitor(Opcodes.ASM9) {
        private lateinit var currentClass: String
        private var currentSuper: String? = null
        private val currentInterfaces = mutableListOf<String>()

        override fun visit(
            version: Int,
            access: Int,
            name: String,
            signature: String?,
            superName: String?,
            interfaces: Array<out String>?,
        ) {
            currentClass = name
            currentSuper = superName
            collectSignatureReferences(signature, references)
            if (superName != null) references += superName
            interfaces?.forEach {
                currentInterfaces += it
                references += it
            }
        }

        override fun visitField(
            access: Int,
            name: String,
            descriptor: String,
            signature: String?,
            value: Any?,
        ): FieldVisitor? {
            if ((access and Opcodes.ACC_STATIC) == 0) {
                fields += WireField(name = name, descriptor = descriptor, typeName = descriptor.toTypeName())
            }
            collectTypeReferences(descriptor, references)
            collectSignatureReferences(signature, references)
            return referenceCollectingFieldVisitor(references)
        }

        override fun visitAnnotation(descriptor: String, visible: Boolean): AnnotationVisitor =
            referenceCollectingAnnotationVisitor(descriptor, references)

        override fun visitRecordComponent(
            name: String,
            descriptor: String,
            signature: String?,
        ): RecordComponentVisitor {
            collectTypeReferences(descriptor, references)
            collectSignatureReferences(signature, references)
            return object : RecordComponentVisitor(Opcodes.ASM9) {
                override fun visitAnnotation(descriptor: String, visible: Boolean): AnnotationVisitor =
                    referenceCollectingAnnotationVisitor(descriptor, references)
            }
        }

        override fun visitMethod(
            access: Int,
            name: String,
            descriptor: String,
            signature: String?,
            exceptions: Array<out String>?,
        ): MethodVisitor = object : MethodVisitor(Opcodes.ASM9) {
            init {
                collectTypeReferences(descriptor, references)
                collectSignatureReferences(signature, references)
                exceptions?.forEach { references += it }
            }

            override fun visitAnnotation(descriptor: String, visible: Boolean): AnnotationVisitor =
                referenceCollectingAnnotationVisitor(descriptor, references)

            override fun visitTypeInsn(opcode: Int, type: String) {
                references += type
            }

            override fun visitFieldInsn(opcode: Int, owner: String, name: String, descriptor: String) {
                references += owner
                collectTypeReferences(descriptor, references)
            }

            override fun visitMethodInsn(opcode: Int, owner: String, name: String, descriptor: String, isInterface: Boolean) {
                references += owner
                collectTypeReferences(descriptor, references)
            }

            override fun visitInvokeDynamicInsn(
                name: String,
                descriptor: String,
                bootstrapMethodHandle: Handle,
                vararg bootstrapMethodArguments: Any,
            ) {
                collectTypeReferences(descriptor, references)
                collectHandleReferences(bootstrapMethodHandle, references)
                bootstrapMethodArguments.forEach { collectConstantReferences(it, references) }
            }

            override fun visitLdcInsn(value: Any?) {
                collectConstantReferences(value, references)
            }

            override fun visitLocalVariable(
                name: String,
                descriptor: String,
                signature: String?,
                start: org.objectweb.asm.Label?,
                end: org.objectweb.asm.Label?,
                index: Int,
            ) {
                collectTypeReferences(descriptor, references)
                collectSignatureReferences(signature, references)
            }
        }

        fun parsed(): ParsedClass = ParsedClass(
            internalName = currentClass,
            superName = currentSuper,
            interfaces = currentInterfaces.toList(),
            fields = fields.toList(),
            referencedInternalNames = references.filter { it != currentClass }.toSortedSet(),
        )
    }
    reader.accept(visitor, ClassReader.SKIP_FRAMES)
    return visitor.parsed()
}

private fun collectTypeReferences(descriptor: String, output: MutableSet<String>) {
    runCatching { Type.getType(descriptor) }.getOrNull()?.collectReferences(output)
        ?: runCatching { Type.getArgumentTypes(descriptor).forEach { it.collectReferences(output) } }
    runCatching { Type.getReturnType(descriptor).collectReferences(output) }
}

private fun collectSignatureReferences(signature: String?, output: MutableSet<String>) {
    if (signature == null) return
    runCatching {
        SignatureReader(signature).accept(object : SignatureVisitor(Opcodes.ASM9) {
            override fun visitClassType(name: String) {
                output += name
            }

            override fun visitInnerClassType(name: String) {
                output += name
            }
        })
    }
}

private fun collectConstantReferences(value: Any?, output: MutableSet<String>) {
    when (value) {
        is Type -> collectTypeReferences(value.descriptor, output)
        is Handle -> collectHandleReferences(value, output)
    }
}

private fun collectHandleReferences(handle: Handle, output: MutableSet<String>) {
    output += handle.owner
    collectTypeReferences(handle.desc, output)
}

private fun referenceCollectingFieldVisitor(output: MutableSet<String>): FieldVisitor = object : FieldVisitor(Opcodes.ASM9) {
    override fun visitAnnotation(descriptor: String, visible: Boolean): AnnotationVisitor =
        referenceCollectingAnnotationVisitor(descriptor, output)
}

private fun referenceCollectingAnnotationVisitor(descriptor: String, output: MutableSet<String>): AnnotationVisitor {
    collectTypeReferences(descriptor, output)
    return object : AnnotationVisitor(Opcodes.ASM9) {
        override fun visit(name: String?, value: Any?) {
            collectConstantReferences(value, output)
        }

        override fun visitEnum(name: String?, descriptor: String, value: String?) {
            collectTypeReferences(descriptor, output)
        }

        override fun visitAnnotation(name: String?, descriptor: String): AnnotationVisitor =
            referenceCollectingAnnotationVisitor(descriptor, output)

        override fun visitArray(name: String?): AnnotationVisitor = this
    }
}

private fun Type.collectReferences(output: MutableSet<String>) {
    when (sort) {
        Type.ARRAY -> elementType.collectReferences(output)
        Type.OBJECT -> output += internalName
        Type.METHOD -> {
            argumentTypes.forEach { it.collectReferences(output) }
            returnType.collectReferences(output)
        }
    }
}

private fun String.toTypeName(): String = when (this) {
    "Z" -> "Boolean"
    "B" -> "Byte"
    "C" -> "Char"
    "S" -> "Short"
    "I" -> "Int"
    "J" -> "Long"
    "F" -> "Float"
    "D" -> "Double"
    else -> runCatching { Type.getType(this).className }.getOrDefault(this)
}
