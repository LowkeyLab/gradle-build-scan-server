package build.scan.tools.develocityschemainfer;

import java.nio.file.Path;

public final class Main {
    private Main() {}

    public static void main(String[] args) throws Exception {
        if (args.length < 2 || args.length > 3) {
            System.err.println("Usage: develocity-schema-infer <plugin.jar> <output-directory> [coordinates]");
            System.exit(2);
        }
        String coordinates = args.length == 3 ? args[2] : "com.gradle:develocity-gradle-plugin:4.4.2";
        SchemaInferenceTool.run(Path.of(args[0]), Path.of(args[1]), coordinates);
    }
}
