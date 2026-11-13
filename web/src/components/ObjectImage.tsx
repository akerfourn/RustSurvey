import { useEffect, useState } from "preact/hooks";

function ImageRequest({ src, alt }: { src: string; alt: string }) {
  const [attempt, setAttempt] = useState(0);
  const [failed, setFailed] = useState(false);
  useEffect(() => {
    if (!failed || attempt >= 3) return;
    const timer = window.setTimeout(
      () => {
        setFailed(false);
        setAttempt((value) => value + 1);
      },
      600 * (attempt + 1),
    );
    return () => window.clearTimeout(timer);
  }, [failed, attempt]);
  function handleError(): void {
    setFailed(true);
  }
  let url = src;
  if (attempt > 0) url = `${src}?retry=${attempt}`;
  if (failed && attempt >= 3)
    return (
      <p className="warning" role="status">
        Image indisponible : {alt}
      </p>
    );
  return <img src={url} alt={alt} loading="lazy" decoding="async" onError={handleError} />;
}

export function ObjectImage({ src, alt }: { src: string; alt: string }) {
  return <ImageRequest key={src} src={src} alt={alt} />;
}
