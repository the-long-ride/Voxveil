interface ScreenIntroProps {
  id: string;
  title: string;
  description: string;
}

export function ScreenIntro({ id, title, description }: ScreenIntroProps) {
  return (
    <div className="screen-intro">
      <h1 id={id}>{title}</h1>
      <p>{description}</p>
    </div>
  );
}
