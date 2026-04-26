declare module "d3" {
  export interface ZoomTransform {
    x: number;
    y: number;
    k: number;
    scale(scale: number): ZoomTransform;
    toString(): string;
    translate(x: number, y: number): ZoomTransform;
  }

  export const zoomIdentity: ZoomTransform;

  export interface Selection<GElement extends Element, Datum> {
    attr(
      name: string,
      value: string | number | null,
    ): Selection<GElement, Datum>;
    call(
      fn: (selection: Selection<GElement, Datum>) => unknown,
    ): Selection<GElement, Datum>;
    call<Arg>(
      fn: (selection: Selection<GElement, Datum>, arg: Arg) => unknown,
      arg: Arg,
    ): Selection<GElement, Datum>;
    call(
      fn: (
        selection: Selection<GElement, Datum>,
        ...args: readonly unknown[]
      ) => unknown,
      ...args: readonly unknown[]
    ): Selection<GElement, Datum>;
    on(typenames: string, listener: null): Selection<GElement, Datum>;
  }

  export interface ZoomBehavior<GElement extends Element, Datum> {
    (selection: Selection<GElement, Datum>): void;
    extent(extent: [[number, number], [number, number]]): this;
    on(
      typenames: string,
      listener: (event: D3ZoomEvent<GElement, Datum>) => void,
    ): this;
    scaleExtent(extent: readonly [number, number]): this;
    transform(
      selection: Selection<GElement, Datum>,
      transform: ZoomTransform,
    ): void;
    translateExtent(extent: [[number, number], [number, number]]): this;
  }

  export interface D3ZoomEvent<GElement extends Element, Datum> {
    sourceEvent: Event;
    target: ZoomBehavior<GElement, Datum>;
    transform: ZoomTransform;
    type: string;
  }

  export function select<GElement extends Element>(
    element: GElement,
  ): Selection<GElement, unknown>;

  export function zoom<GElement extends Element, Datum>(): ZoomBehavior<
    GElement,
    Datum
  >;
}
