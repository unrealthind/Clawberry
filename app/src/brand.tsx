export function Brand({ large = false }: { large?: boolean }) {
  return (
    <div className={`brand${large ? " brand-large" : ""}`}>
      <img
        className="brand-mark"
        src="/brand/mark.svg"
        alt=""
        width="32"
        height="32"
      />
      <span>Clawberry</span>
    </div>
  );
}
