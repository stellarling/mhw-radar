const titleStyle: React.CSSProperties = {
  color: "#dcdcdc",
  fontSize: 16,
  margin: "0 0 4px",
};

const descStyle: React.CSSProperties = {
  color: "#8c8c8c",
  fontSize: 12,
  margin: "0 0 16px",
};

export function SectionHeader({ title, description }: { title: string; description: string }) {
  return (
    <>
      <h2 style={titleStyle}>{title}</h2>
      <p style={descStyle}>{description}</p>
    </>
  );
}
