"use client";

import { useState } from "react";
import { Check, Copy, Eye, EyeOff, Key, RefreshCcw } from "lucide-react";

import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogTitle,
  DialogTrigger,
} from "@/components/ui/dialog";

type Props = {
  setPassword: (password: string) => void;
  onOpenChange: (value: boolean) => void;
  isModalOpened: boolean;
};

export function StrongPasswordDialog({
  onOpenChange,
  isModalOpened,
  setPassword,
}: Props) {
  const [isCopied, setIsCopied] = useState(false);
  const [strongPassword, setStrongPassword] = useState("");
  const [isPasswordVisible, setIsPasswordVisible] = useState(false);

  function copyToClipboard() {
    navigator.clipboard.writeText(strongPassword);
    setIsCopied(true);

    setTimeout(() => setIsCopied(false), 1000);
  }

  function generatePassword(): string {
    const passwordLength = 32;
    const charSet =
      "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789!@#$%^&*()_+-=[]{}|;:,.<>?";

    let password = "";
    for (let i = 0; i < passwordLength; i++) {
      const randomIndex = Math.floor(Math.random() * charSet.length);
      password += charSet[randomIndex];
    }

    return password;
  }

  function handler() {
    setPassword(strongPassword);
    onOpenChange(false);
  }

  return (
    <Dialog open={isModalOpened} onOpenChange={onOpenChange}>
      <DialogTrigger
        type="button"
        className="text-sm text-blue-500 underline-offset-2 hover:underline"
      >
        Generate a strong password
      </DialogTrigger>

      <DialogContent className="min-w-[38rem]">
        <DialogTitle>Generate a strong password</DialogTitle>

        <DialogDescription>
          A strong and random password helps protect your online accounts and
          personal information from cyber threats
        </DialogDescription>

        <DialogDescription>
          By using a mix of characters and symbols, you can reduce the risk of
          unauthorized access and identity theft.
        </DialogDescription>

        <DialogDescription>
          Protect the application by choosing a strong password.
        </DialogDescription>

        <div className="bg-background border border-gray-200 dark:border-gray-700 dark:bg- mt-4 flex items-center justify-between rounded-[10px] p-2 px-4 shadow-inner brightness-90">
          <strong className="text-primary text-[1.15rem] font-bold tracking-widest">
            {isPasswordVisible
              ? strongPassword
              : strongPassword
                  .split("")
                  .map(() => "*")
                  .join("")}
          </strong>

          <div className="flex items-center gap-3">
            <button
              type="button"
              onClick={() => setIsPasswordVisible((state) => !state)}
              title="Show password"
              className="flex w-fit items-center justify-center"
            >
              {isPasswordVisible ? <EyeOff size={28} /> : <Eye size={28} />}
            </button>

            <button
              type="button"
              onClick={() => setStrongPassword(generatePassword())}
              title="Regenerate the password"
              className="flex w-fit items-center justify-center transition-all active:scale-90"
            >
              <RefreshCcw size={28} />
            </button>
          </div>
        </div>

        <div className="ml-auto flex gap-2">
          <Button variant="outline" type="button" onClick={copyToClipboard}>
            {isCopied ? (
              <>
                <Check size={28} />
                Copied
              </>
            ) : (
              <>
                <Copy size={28} />
                Copy
              </>
            )}
          </Button>

          <Button type="button" onClick={handler}>
            <Key size={28} />
            Use this password
          </Button>
        </div>
      </DialogContent>
    </Dialog>
  );
}
