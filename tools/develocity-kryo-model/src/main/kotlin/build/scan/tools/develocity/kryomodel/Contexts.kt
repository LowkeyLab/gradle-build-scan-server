package build.scan.tools.develocity.kryomodel

import arrow.core.raise.Raise
import java.nio.file.Path

interface JarReadContext {
    context(raise: Raise<DiscoveryError>)
    suspend fun readClassEntries(jar: Path): List<JarClassEntry>
}

interface BytecodeScanContext {
    context(raise: Raise<DiscoveryError>)
    suspend fun parseClass(entry: JarClassEntry): ParsedClass
}

interface SourceWriteContext {
    context(raise: Raise<GenerationError>)
    suspend fun writeSource(relativePath: String, content: String)
}

interface GenerationContext : JarReadContext, BytecodeScanContext, SourceWriteContext

data class JarClassEntry(
    val name: String,
    val bytes: ByteArray,
) {
    override fun equals(other: Any?): Boolean = other is JarClassEntry && name == other.name && bytes.contentEquals(other.bytes)
    override fun hashCode(): Int = 31 * name.hashCode() + bytes.contentHashCode()
}

data class ParsedClass(
    val internalName: String,
    val superName: String?,
    val interfaces: List<String>,
    val fields: List<WireField>,
    val referencedInternalNames: Set<String>,
)
