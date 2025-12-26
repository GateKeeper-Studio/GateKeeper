"use client";

import * as React from "react";
import { useTheme } from "next-themes";
import { RadioGroup, RadioGroupItem } from "@/components/ui/radio-group";
import { Label } from "@/components/ui/label";

export function ThemeSwitcher() {
  const { theme, setTheme } = useTheme();

  return (
    <RadioGroup
      value={theme}
      onValueChange={(val) => setTheme(val)}
      className="flex gap-4 mt-4  w-full relative"
    >
      <div className="relative flex flex-col gap-4 w-full max-w-[33.33%]">
        <RadioGroupItem
          id="theme-light"
          value="light"
          className="peer sr-only"
        />

        <Label
          htmlFor="theme-light"
          className="cursor-pointer rounded-lg border shadow w-full h-[190px] p-2 flex gap-3 transition-all peer-data-[state=checked]:border-primary peer-data-[state=checked]:ring-6 peer-data-[state=checked]:ring-primary/30 bg-white"
        >
          <div className="h-full bg-gray-200 w-[20%] rounded-md" />
          <div className="w-full flex flex-col gap-2 ">
            <div className="h-3 w-[33%] bg-gray-200 rounded" />
            <div className="h-3 w-[80%] bg-gray-200 rounded" />
            <div className="h-8 w-full bg-gray-200 rounded" />
            <div className="flex gap-2 relative h-[30px]">
              <div className="h-full w-[70%] bg-gray-200 rounded" />
              <div className="h-full w-[70%] bg-gray-200 rounded" />
              <div className="h-full w-[70%] bg-gray-200 rounded" />
            </div>
            <div className="h-3 w-[60%] bg-gray-200 rounded" />
            <div className="h-3 w-[90%] bg-gray-200 rounded" />
          </div>
        </Label>

        <span className="text-lg font-semibold">Light</span>
      </div>

      <div className="relative flex flex-col gap-4 w-full max-w-[33.33%]">
        <RadioGroupItem id="theme-dark" value="dark" className="peer sr-only" />
        <Label
          htmlFor="theme-dark"
          className="cursor-pointer rounded-lg border border-gray-500 shadow w-full h-[190px] p-3 flex gap-2 bg-gray-900 transition-all peer-data-[state=checked]:border-primary peer-data-[state=checked]:ring-6 peer-data-[state=checked]:ring-primary/30"
        >
          <div className="h-full bg-gray-700 w-[20%] rounded-md" />
          <div className="w-full flex flex-col gap-2">
            <div className="h-3 w-[33%] bg-gray-700 rounded" />
            <div className="h-3 w-[80%] bg-gray-700 rounded" />
            <div className="h-8 w-full bg-gray-700 rounded" />
            <div className="flex gap-2 relative h-[30px]">
              <div className="h-full w-[70%] bg-gray-700 rounded" />
              <div className="h-full w-[70%] bg-gray-700 rounded" />
              <div className="h-full w-[70%] bg-gray-700 rounded" />
            </div>
            <div className="h-3 w-[60%] bg-gray-700 rounded" />
            <div className="h-3 w-[90%] bg-gray-700 rounded" />
          </div>
        </Label>

        <span className="text-lg font-semibold">Dark</span>
      </div>

      <div className="relative flex flex-col gap-4 w-full max-w-[33.33%]">
        <RadioGroupItem
          id="theme-system"
          value="system"
          className="peer sr-only"
        />
        <Label
          htmlFor="theme-system"
          className="relative cursor-pointer rounded-lg  shadow-lg w-full h-[190px] p-3 flex gap-2 bg-black transition-all peer-data-[state=checked]:border-primary peer-data-[state=checked]:ring-6 peer-data-[state=checked]:ring-primary/30"
        >
          <div className="h-full bg-gray-800 w-[20%] rounded-md" />
          <div className="w-full flex flex-col gap-2">
            <div className="h-3 w-[33%] bg-gray-800 rounded" />
            <div className="h-3 w-[80%] bg-gray-800 rounded" />
            <div className="h-8 w-full bg-gray-800 rounded" />
            <div className="flex gap-2 relative h-[30px]">
              <div className="h-full w-[70%] bg-gray-800 rounded" />
              <div className="h-full w-[70%] bg-gray-800 rounded" />
              <div className="h-full w-[70%] bg-gray-800 rounded" />
            </div>
            <div className="h-3 w-[60%] bg-gray-800 rounded" />
            <div className="h-3 w-[90%] bg-gray-800 rounded" />
          </div>

          <div className="absolute w-[50%] h-full top-0 left-0 rounded-l-lg backdrop-invert w-50%"></div>
        </Label>

        <span className="text-lg font-semibold">System</span>
      </div>
    </RadioGroup>
  );
}
