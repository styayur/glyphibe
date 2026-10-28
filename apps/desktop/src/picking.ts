import { Raycaster, Sphere, Vector2, Vector3 } from "three";
import type { ForceGraphMethods } from "react-force-graph-3d";

// The rendered nodes explicitly use nodeVal=1 and nodeRelSize=radius. Pick the
// same spheres, using owned coordinates and the public camera/renderer APIs.
// This avoids depending on the renderer's private hover state or object fields.
export function createSpherePicker(
  graph: () => ForceGraphMethods | undefined,
  nodes: readonly { id: string; fx: number; fy: number; fz: number }[],
  radius: number,
) {
  const caster = new Raycaster();
  const point = new Vector2();
  const sphere = new Sphere(new Vector3(), radius);
  const hit = new Vector3();
  return (clientX: number, clientY: number): string | null => {
    const current = graph();
    if (!current) return null;
    const bounds = current.renderer().domElement.getBoundingClientRect();
    if (bounds.width <= 0 || bounds.height <= 0) return null;
    point.set(
      ((clientX - bounds.left) / bounds.width) * 2 - 1,
      1 - ((clientY - bounds.top) / bounds.height) * 2,
    );
    caster.setFromCamera(point, current.camera());
    let nearest = Infinity;
    let selected: string | null = null;
    for (const node of nodes) {
      sphere.center.set(node.fx, node.fy, node.fz);
      if (!caster.ray.intersectSphere(sphere, hit)) continue;
      const distance = caster.ray.origin.distanceToSquared(hit);
      if (distance < nearest) {
        nearest = distance;
        selected = node.id;
      }
    }
    return selected;
  };
}
