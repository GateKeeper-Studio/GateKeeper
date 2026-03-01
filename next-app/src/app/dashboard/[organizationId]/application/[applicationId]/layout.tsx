import type { PropsWithChildren } from "react";
import { ApplicationContextProvider } from "./(contexts)/application-context-provider";

type Props = PropsWithChildren<object>;

export default function Layout({ children }: Props) {
  return <ApplicationContextProvider>{children}</ApplicationContextProvider>;
}
