import { Component, ChangeDetectionStrategy, input } from "@angular/core";
import { DatePipe } from "@angular/common";

@Component({
  selector: "app-build-metadata",
  imports: [DatePipe],
  changeDetection: ChangeDetectionStrategy.OnPush,
  template: `
    @if (scan(); as scan) {
      <div class="card bg-base-100 shadow-xl mb-6">
        <div class="card-body">
          <h2 class="card-title">
            Build Scan
            <span
              class="badge"
              [class.badge-success]="scan.outcome === 'success'"
              [class.badge-error]="scan.outcome === 'failed'"
            >
              {{ scan.outcome }}
            </span>
          </h2>

          <div class="grid grid-cols-2 gap-4 mt-4">
            <div>
              <span class="text-sm opacity-60">Created</span>
              <p>{{ scan.createdAt | date: "long" }}</p>
            </div>
            <div>
              <span class="text-sm opacity-60">Build Tool</span>
              <p>{{ scan.buildToolType }} {{ scan.buildToolVersion }}</p>
            </div>
            <div>
              <span class="text-sm opacity-60">Plugin Version</span>
              <p>{{ scan.pluginVersion }}</p>
            </div>
            <div>
              <span class="text-sm opacity-60">Hostname</span>
              <p>{{ scan.hostname || "—" }}</p>
            </div>
            @if (scan.osName) {
              <div>
                <span class="text-sm opacity-60">OS</span>
                <p>{{ scan.osName }} {{ scan.osVersion }}</p>
              </div>
            }
            @if (scan.jvmVersion) {
              <div>
                <span class="text-sm opacity-60">JVM</span>
                <p>{{ scan.jvmVendor }} {{ scan.jvmVersion }}</p>
              </div>
            }
          </div>

          @if (scan.requestedTasks.length > 0) {
            <div class="mt-4">
              <span class="text-sm opacity-60">Requested Tasks</span>
              <p>{{ scan.requestedTasks.join(" ") }}</p>
            </div>
          }
        </div>
      </div>
    }
  `,
})
export class BuildMetadataComponent {
  scan = input.required<any>();
}
