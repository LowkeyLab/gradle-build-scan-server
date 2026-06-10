package build.scan.tools.develocity.kryomodel

import arrow.core.raise.Raise
import java.nio.file.Path

private const val EVENT_MODEL_PREFIX = "com/gradle/scan/eventmodel/"
private const val KRYO_SERIALIZER_PREFIX = "com/gradle/scan/agent/serialization/scan/serializer/kryo/"
private const val KRYO_REGISTRY = "com/gradle/scan/agent/serialization/scan/serializer/kryo/KryoEventDataSerializerRegistryGradle"

context(jarRead: JarReadContext, bytecodeScan: BytecodeScanContext)
suspend fun Raise<DiscoveryError>.scanDevelocityPluginJar(
    pluginVersion: PluginVersion,
    jar: Path,
): DiscoveryReport {
    val entries = jarRead.readClassEntries(jar)
    if (entries.isEmpty()) raise(DiscoveryError.NoClassesFound(jar.toString()))
    val parsedClasses = entries.map { bytecodeScan.parseClass(it) }
    return discoverKryoWireModel(pluginVersion, parsedClasses)
}

fun discoverKryoWireModel(pluginVersion: PluginVersion, parsedClasses: List<ParsedClass>): DiscoveryReport {
    val eventClassesByName = parsedClasses
        .filter { it.internalName.startsWith(EVENT_MODEL_PREFIX) && !it.internalName.contains('$') }
        .associateBy { it.internalName }
    val kryoClasses = parsedClasses.filter { it.internalName.startsWith(KRYO_SERIALIZER_PREFIX) }
    val registrySerializerNames = kryoClasses
        .firstOrNull { it.internalName == KRYO_REGISTRY }
        ?.referencedInternalNames
        .orEmpty()
        .filter { it.startsWith(KRYO_SERIALIZER_PREFIX) }
        .toSet()
    val bindings = kryoClasses.flatMap { serializer ->
        serializer.referencedInternalNames
            .filter { referenced -> referenced in eventClassesByName }
            .map { referenced -> bindingFor(serializer, referenced, serializer.internalName in registrySerializerNames) }
    }.distinctBy { it.serializerInternalName to it.wireClassInternalName }
        .sortedWith(compareBy<KryoSerializerBinding> { it.wireClassInternalName }.thenBy { it.serializerInternalName })
    val bindingsByWireClass = bindings.groupBy { it.wireClassInternalName }
    val discovered = eventClassesByName.values.map { eventClass ->
        val classBindings = bindingsByWireClass[eventClass.internalName].orEmpty()
        val confidence = confidenceFor(classBindings)
        DiscoveredClass(
            internalName = eventClass.internalName,
            binaryName = eventClass.internalName.toBinaryName(),
            simpleName = eventClass.internalName.substringAfterLast('/'),
            fields = eventClass.fields,
            evidence = baseEvidence(eventClass) + classBindings.flatMap { it.evidence },
            confidence = confidence,
        )
    }.sortedBy { it.internalName }
    val wireClasses = discovered.filter { it.confidence.isGenerated }.map { discoveredClass ->
        val classBindings = bindingsByWireClass.getValue(discoveredClass.internalName)
        WireClass(
            pluginVersion = pluginVersion,
            internalName = discoveredClass.internalName,
            binaryName = discoveredClass.binaryName,
            simpleName = discoveredClass.simpleName,
            fields = discoveredClass.fields,
            serializerBindings = classBindings,
            evidence = discoveredClass.evidence,
            confidence = discoveredClass.confidence,
        )
    }
    return DiscoveryReport(pluginVersion, discovered, bindings, wireClasses)
}

fun filterConfident(discoveredClasses: List<DiscoveredClass>): List<DiscoveredClass> =
    discoveredClasses.filter { it.confidence.isGenerated }.sortedBy { it.internalName }

private fun bindingFor(serializer: ParsedClass, wireClassInternalName: String, registeredByKryoRegistry: Boolean): KryoSerializerBinding {
    val evidenceKind = if (registeredByKryoRegistry) {
        EvidenceKind.RegistryReferencesEventModel
    } else {
        EvidenceKind.SerializerReferencesEventModel
    }
    // High confidence intentionally requires all three criteria:
    // 1. the serializer class is in the Kryo scan serializer package (selected by kryoClasses),
    // 2. that serializer references an event model class (the binding target), and
    // 3. KryoEventDataSerializerRegistryGradle references the serializer.
    val confidence = if (registeredByKryoRegistry) Confidence.High else Confidence.Medium
    return KryoSerializerBinding(
        wireClassInternalName = wireClassInternalName,
        serializerInternalName = serializer.internalName,
        evidence = listOf(
            DiscoveryEvidence(
                kind = EvidenceKind.KryoSerializerPackage,
                sourceInternalName = serializer.internalName,
                detail = "Class is in the Develocity Kryo scan serializer package.",
            ),
            DiscoveryEvidence(
                kind = evidenceKind,
                sourceInternalName = serializer.internalName,
                detail = if (registeredByKryoRegistry) {
                    "Serializer bytecode references ${wireClassInternalName.toBinaryName()} and the serializer is registered by KryoEventDataSerializerRegistryGradle."
                } else {
                    "Serializer bytecode references ${wireClassInternalName.toBinaryName()} but registry linkage was not found."
                },
            ),
        ),
        confidence = confidence,
    )
}

private fun confidenceFor(bindings: List<KryoSerializerBinding>): Confidence = when {
    bindings.any { it.confidence == Confidence.High } -> Confidence.High
    bindings.isNotEmpty() -> Confidence.Medium
    else -> Confidence.Low
}

private fun baseEvidence(eventClass: ParsedClass): List<DiscoveryEvidence> = listOf(
    DiscoveryEvidence(
        kind = EvidenceKind.EventModelClass,
        sourceInternalName = eventClass.internalName,
        detail = "Class is under com.gradle.scan.eventmodel in the plugin jar.",
    ),
)

internal fun String.toBinaryName(): String = replace('/', '.')
