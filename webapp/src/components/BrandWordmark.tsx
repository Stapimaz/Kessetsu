type BrandWordmarkProps = {
  className?: string;
};

export function BrandWordmark({ className = '' }: BrandWordmarkProps) {
  return (
    <span className={`brand-wordmark ${className}`.trim()} data-testid="brand-wordmark">
      <span className="brand-wordmark-text">kessetsu</span>
      <span className="brand-wordmark-signal" aria-hidden="true">
        <span className="brand-wordmark-node" />
      </span>
    </span>
  );
}
