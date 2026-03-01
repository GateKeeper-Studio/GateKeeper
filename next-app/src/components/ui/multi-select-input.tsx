import { useRef, useState } from "react";

import { Badge } from "./badge";
import { Tooltip, TooltipContent, TooltipTrigger } from "./tooltip";

type Props = {
  items: string[];
  onChange: (items: string[]) => void;
  placeholder?: string;
};

export function MultiSelectInput({ items, onChange, placeholder }: Props) {
  const inputRef = useRef<HTMLInputElement | null>(null);

  const [errorMessage, setErrorMessage] = useState("");
  const [value, setValue] = useState("");

  return (
    <div className="flex flex-col gap-2">
      <div
        onClick={() => inputRef.current?.focus()}
        className="hover:cursor-text flex-wrap gap-1 border-input bg-background ring-offset-background placeholder:text-muted-foreground focus-visible:ring-ring flex min-h-10 w-full rounded-md border px-3 py-2 text-base file:border-0 file:bg-transparent file:text-sm file:font-medium focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-offset-2 disabled:cursor-not-allowed disabled:opacity-50 md:text-sm"
      >
        {items.map((item) => (
          <Tooltip key={item}>
            <TooltipTrigger>
              <Badge
                variant="default"
                className="whitespace-normal"
                onClick={(e) => {
                  e.stopPropagation();
                  onChange(items.filter((i) => i !== item));
                }}
              >
                {item}
              </Badge>
            </TooltipTrigger>

            <TooltipContent>
              <p>Click to delete</p>
            </TooltipContent>
          </Tooltip>
        ))}

        <input
          ref={inputRef}
          placeholder={placeholder || "Type and press enter to add"}
          type="text"
          className="min-w-[175px] outline-none"
          value={value}
          onChange={(e) => setValue(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter") {
              e.preventDefault();

              if (value === "") {
                setErrorMessage("Item is empty");
                setTimeout(() => setErrorMessage(""), 2000);
                return;
              }

              if (!items.includes(value)) {
                onChange([...items, value]);
                setValue("");
              } else {
                setErrorMessage("Item already exists");
                setTimeout(() => setErrorMessage(""), 2000);
              }

              inputRef.current?.focus();
            }
          }}
        />
      </div>

      {errorMessage !== "" && (
        <p className="text-red-500 text-sm">{errorMessage}</p>
      )}
    </div>
  );
}
