export function SectionHeader({ title, description }: { title: string; description: string }) {
  return (
    <>
      <h2 className="mt-0 mb-1 text-base font-semibold text-panel-text">{title}</h2>
      {description && <p className="mt-0 mb-4 text-xs text-panel-muted">{description}</p>}
    </>
  );
}
