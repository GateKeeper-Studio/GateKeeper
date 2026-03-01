import { GithubLogo } from "./github-logo";
import { GoogleLogo } from "./google-logo";

export type OAuthProvider = {
  id: string;
  logo: React.ElementType;
  name: string;
  description: string;
  isEnabled: boolean;
  inputs: {
    id: string;
    label: string;
    placeholder: string;
    type: "text" | "password";
    required: boolean;
    value: string | null;
  }[];
};

export const OAuthProviders: OAuthProvider[] = [
  {
    id: "google",
    logo: GoogleLogo,
    name: "Google",
    description: "Authenticate using Google",
    isEnabled: false,
    inputs: [
      {
        id: "google-client-id",
        label: "Client ID",
        placeholder: "Enter your Google Client ID",
        type: "text",
        required: true,
        value: null,
      },
      {
        id: "google-client-secret",
        label: "Client Secret",
        placeholder: "Enter your Google Client Secret",
        type: "password",
        required: true,
        value: null,
      },
      {
        id: "google-redirect-uri",
        label: "Redirect URI",
        placeholder: "Enter your Google Redirect URI",
        type: "text",
        required: true,
        value: null,
      },
    ],
  },
  {
    id: "github",
    name: "GitHub",
    logo: GithubLogo,
    description: "Authenticate using GitHub",
    isEnabled: false,
    inputs: [
      {
        id: "github-client-id",
        label: "Client ID",
        placeholder: "Enter your GitHub Client ID",
        type: "text",
        required: true,
        value: null,
      },
      {
        id: "github-client-secret",
        label: "Client Secret",
        placeholder: "Enter your GitHub Client Secret",
        type: "password",
        required: true,
        value: null,
      },
      {
        id: "github-redirect-uri",
        label: "Redirect URI",
        placeholder: "Enter your GitHub Redirect URI",
        type: "text",
        required: true,
        value: null,
      },
    ],
  },
];
