import { useEffect, useRef } from "react";

export interface MenuItemDef {
  icon?: string;
  label: string;
  onClick: () => void;
  danger?: boolean;
  separator?: boolean;
}

export type MenuItem = MenuItemDef;

interface Props {
  x: number;
  y: number;
  items: MenuItemDef[];
  onClose: () => void;
}

export function ContextMenu({ x, y, items, onClose }: Props) {
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    function handleClick(e: MouseEvent) {
      if (ref.current && !ref.current.contains(e.target as Node)) {
        onClose();
      }
    }
    function handleKey(e: KeyboardEvent) {
      if (e.key === "Escape") onClose();
    }
    document.addEventListener("mousedown", handleClick);
    document.addEventListener("keydown", handleKey);
    return () => {
      document.removeEventListener("mousedown", handleClick);
      document.removeEventListener("keydown", handleKey);
    };
  }, [onClose]);

  // Adjust position to stay within viewport
  const adjustedX = Math.min(x, window.innerWidth - 230);
  const adjustedY = Math.min(y, window.innerHeight - items.length * 36 - 20);

  return (
    <div
      ref={ref}
      className="ctx"
      style={{ left: adjustedX, top: adjustedY }}
    >
      {items.map((item, i) =>
        item.separator ? (
          <hr key={i} />
        ) : (
          <button
            key={i}
            className={item.danger ? "danger" : undefined}
            onClick={() => {
              item.onClick();
              onClose();
            }}
          >
            {item.icon && (
              <svg className="icon" aria-hidden="true" viewBox="0 0 24 24">
                <path d={item.icon} fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" />
              </svg>
            )}
            {item.label}
          </button>
        )
      )}
    </div>
  );
}
