package build.scan.tools.develocity.kryomodel

import arrow.core.raise.either
import java.nio.file.Path
import kotlinx.coroutines.runBlocking

fun main(args: Array<String>) = runBlocking {
    require(args.size == 2) { "usage: GenerateMain <develocity-plugin-jar> <output-root>" }
    val jar = Path.of(args[0])
    val outputRoot = Path.of(args[1])
    val discovery = either {
        with(JdkJarReadContext()) {
            with(AsmBytecodeScanContext()) {
                scanDevelocityPluginJar(PluginVersion("4.4.2"), jar)
            }
        }
    }.fold(
        { error -> error("Discovery failed: $error") },
        { it },
    )
    val sources = either { renderKotlinSources(discovery.wireClasses) }.fold(
        { error -> error("Rendering failed: $error") },
        { it },
    )
    val writer = JdkGenerationContext(outputRoot)
    either {
        with(writer) {
            sources.forEach { (relativePath, content) -> writeSource(relativePath, content) }
        }
    }.fold(
        { error -> error("Writing failed: $error") },
        { },
    )
    println("Generated ${discovery.wireClasses.size} confident wire classes")
}
