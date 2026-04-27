val syntheticTaskCount = providers
    .gradleProperty("syntheticTaskCount")
    .orNull
    ?.toIntOrNull()
    ?: 0

val syntheticTaskLayerWidth = providers
    .gradleProperty("syntheticTaskLayerWidth")
    .orNull
    ?.toIntOrNull()
    ?: 40

val syntheticTaskFanIn = providers
    .gradleProperty("syntheticTaskFanIn")
    .orNull
    ?.toIntOrNull()
    ?: 3

if (syntheticTaskCount > 0) {
    val taskCount = syntheticTaskCount.coerceAtMost(10_000)
    val layerWidth = syntheticTaskLayerWidth.coerceAtLeast(1)
    val fanIn = syntheticTaskFanIn.coerceAtLeast(1)

    fun taskName(index: Int): String = "syntheticTask${index.toString().padStart(4, '0')}"

    fun dependencyIndexes(index: Int): List<Int> {
        val layer = index / layerWidth
        if (layer == 0) {
            return emptyList()
        }

        val previousLayerStart = (layer - 1) * layerWidth
        val previousLayerEndExclusive = minOf(layer * layerWidth, taskCount)
        val previousLayerSize = previousLayerEndExclusive - previousLayerStart
        val dependencyCount = minOf(fanIn, previousLayerSize)

        return (0 until dependencyCount)
            .map { offset ->
                previousLayerStart + ((index + (offset * 7)) % previousLayerSize)
            }
            .distinct()
    }

    val syntheticTasks = (0 until taskCount).associateWith { index ->
        tasks.register(taskName(index)) {
            group = "verification"
            description = "Synthetic task ${index + 1} of $taskCount for build scan performance testing"
            outputs.upToDateWhen { false }
            doLast {
                // Intentionally empty: we only need a deterministic execution graph.
            }
        }
    }

    syntheticTasks.forEach { (index, taskProvider) ->
        taskProvider.configure {
            dependencyIndexes(index).forEach { dependencyIndex ->
                dependsOn(syntheticTasks.getValue(dependencyIndex))
            }
        }
    }

    tasks.register("syntheticTaskGraph") {
        group = "verification"
        description = "Executes a deterministic synthetic DAG for build scan performance testing"

        val finalLayerStart = ((taskCount - 1) / layerWidth) * layerWidth
        dependsOn(
            (finalLayerStart until taskCount).map { index -> syntheticTasks.getValue(index) },
        )

        doLast {
            println(
                "Executed synthetic task graph with $taskCount tasks, layer width $layerWidth, and fan-in $fanIn.",
            )
        }
    }
}
