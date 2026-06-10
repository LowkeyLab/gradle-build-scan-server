package build.scan.tools.develocity.kryomodel

@JvmInline
value class PluginVersion(val value: String) {
    init {
        require(value.isNotBlank()) { "plugin version must not be blank" }
    }
}

data class DiscoveredClass(
    val internalName: String,
    val binaryName: String,
    val simpleName: String,
    val fields: List<WireField>,
    val evidence: List<DiscoveryEvidence>,
    val confidence: Confidence,
)

data class KryoSerializerBinding(
    val wireClassInternalName: String,
    val serializerInternalName: String,
    val evidence: List<DiscoveryEvidence>,
    val confidence: Confidence,
)

data class WireClass(
    val pluginVersion: PluginVersion,
    val internalName: String,
    val binaryName: String,
    val simpleName: String,
    val fields: List<WireField>,
    val serializerBindings: List<KryoSerializerBinding>,
    val evidence: List<DiscoveryEvidence>,
    val confidence: Confidence,
)

data class WireField(
    val name: String,
    val descriptor: String,
    val typeName: String,
    val nullable: Boolean = true,
)

data class DiscoveryEvidence(
    val kind: EvidenceKind,
    val sourceInternalName: String,
    val detail: String,
)

enum class EvidenceKind {
    EventModelClass,
    KryoSerializerPackage,
    SerializerReferencesEventModel,
    RegistryReferencesEventModel,
}

enum class Confidence {
    Low,
    Medium,
    High;

    val isGenerated: Boolean
        get() = this == High
}

data class DiscoveryReport(
    val pluginVersion: PluginVersion,
    val discoveredClasses: List<DiscoveredClass>,
    val serializerBindings: List<KryoSerializerBinding>,
    val wireClasses: List<WireClass>,
)

sealed interface DiscoveryError {
    data class JarReadFailed(val path: String, val message: String) : DiscoveryError
    data class ClassReadFailed(val entryName: String, val message: String) : DiscoveryError
    data class NoClassesFound(val path: String) : DiscoveryError
    data class InvalidInput(val message: String) : DiscoveryError
}

sealed interface GenerationError {
    data class EmptyPackage(val message: String) : GenerationError
    data class InvalidClassName(val internalName: String) : GenerationError
    data class SourceWriteFailed(val relativePath: String, val message: String) : GenerationError
}
