import { useErrorBoundary } from "preact/hooks";
import type { ComponentChildren } from "preact";

interface ViewerBoundaryProps {
  children: ComponentChildren;
}

export function ViewerBoundary({ children }: ViewerBoundaryProps) {
  const [error] = useErrorBoundary();
  if (error !== undefined) {
    return (
      <p className="notice error" role="alert">
        Le viewer n’a pas pu afficher cette étude. Vous pouvez analyser un autre fichier ci-dessus.
      </p>
    );
  }
  return <>{children}</>;
}
