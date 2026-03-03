type GateKeeperSession = {
  user: {
    id: string;
    firstName: string;
    lastName: string;
    email: string;
    displayName: string;
    applicationId: string;
  };
  accessToken: string;
  idToken?: string;
  refreshToken?: string;
};

type GateKeeperSessionError = {
  message: string;
};
