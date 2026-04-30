declare module "@antv/layout-wasm/dist/index.min.js" {
  export {
    AntVDagreLayout,
    initThreads,
    supportsThreads,
    type Threads,
  } from "@antv/layout-wasm";

  import {
    AntVDagreLayout,
    initThreads,
    supportsThreads,
  } from "@antv/layout-wasm";

  const layoutWasm: {
    AntVDagreLayout: typeof AntVDagreLayout;
    initThreads: typeof initThreads;
    supportsThreads: typeof supportsThreads;
  };
  export default layoutWasm;
}
