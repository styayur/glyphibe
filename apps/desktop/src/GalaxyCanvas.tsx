import { useEffect } from "react";
import type { MutableRefObject } from "react";
import ForceGraph3D from "react-force-graph-3d";
import type { ForceGraphMethods, ForceGraphProps } from "react-force-graph-3d";

interface Props extends ForceGraphProps {
  graphRef: MutableRefObject<ForceGraphMethods | undefined>;
  onReady: (ready: boolean) => void;
}

// This component itself is lazy. Its effect runs after the renderer's
// imperative ref is committed, including the very first Galaxy transition.
export default function GalaxyCanvas({ graphRef, onReady, ...props }: Props) {
  useEffect(() => {
    onReady(true);
    return () => onReady(false);
  }, [onReady]);
  return <ForceGraph3D ref={graphRef} {...props} />;
}
