"use client";

import {
  createContext,
  useContext,
  useState,
  useCallback,
  type ReactNode,
} from "react";

type StepUpContextType = {
  stepUpToken: string | null;
  requestStepUp: () => Promise<string | null>;
  clearStepUp: () => void;
};

const StepUpContext = createContext({} as StepUpContextType);

type StepUpProviderProps = {
  children: ReactNode;
  onRequestReauth: () => Promise<string | null>;
};

export function StepUpProvider({
  children,
  onRequestReauth,
}: StepUpProviderProps) {
  const [stepUpToken, setStepUpToken] = useState<string | null>(null);

  const requestStepUp = useCallback(async () => {
    const token = await onRequestReauth();
    if (token) {
      setStepUpToken(token);
    }
    return token;
  }, [onRequestReauth]);

  const clearStepUp = useCallback(() => {
    setStepUpToken(null);
  }, []);

  return (
    <StepUpContext.Provider value={{ stepUpToken, requestStepUp, clearStepUp }}>
      {children}
    </StepUpContext.Provider>
  );
}

export const useStepUp = () => useContext(StepUpContext);
