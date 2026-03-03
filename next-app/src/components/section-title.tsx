type Props = {
  children: React.ReactNode;
};

export function SectionTitle({ children }: Props) {
  return (
    <div className="flex gap-3 items-center">
      <div className="h-8 w-2.5 bg-primary rounded-r-md" />
      <h3 className="text-xl">{children}</h3>
    </div>
  );
}
