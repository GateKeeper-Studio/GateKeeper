"use client";

import { useRef, useState } from "react";

import { generateMfaToken } from "@/lib/utils/generate-mfa-token";
import { generateQrCode } from "@/lib/utils/generate-qrcode";
import { confirmMfaUserSecret } from "@/lib/utils/confirm-mfa-user-secret";

export function GenerateMfaSecretButton() {
  const qrCodeCanvas = useRef<HTMLCanvasElement | null>(null);
  const [mfaCode, setMfaCode] = useState("");

  function generateMfaTokenHandler() {
    if (qrCodeCanvas.current) {
      generateMfaToken()
        .then((data) => {
          const qrCode = generateQrCode(
            data.otpUrl,
            qrCodeCanvas.current as HTMLCanvasElement
          );

          console.log("QR Code generated:", qrCode);
        })
        .catch((error: Error) => {
          console.error("Error generating MFA token:", error);

          alert(error.message || "Error generating MFA token. Please try again.");
        });
    }
  }

  return (
    <div className="flex flex-col items-center justify-center p-4 bg-white rounded-md shadow-md">
      <button
        onClick={generateMfaTokenHandler}
        type="button"
        className="px-4 py-2 text-white bg-blue-500 rounded-md shadow-md hover:bg-blue-600 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 focus:ring-offset-white"
      >
        Generate MFA Secret
      </button>

      <div className="mt-4 bg-red-50">
        <canvas
          ref={qrCodeCanvas}
          id="qrcode"
          width={256}
          height={256}
          className="border border-gray-300 rounded-md"
        ></canvas>
      </div>

      <input
        type="text"
        placeholder="Enter MFA Auth App Code"
        value={mfaCode}
        onChange={(e) => setMfaCode(e.target.value)}
        className="mt-4 px-4 py-2 border border-gray-300 rounded-md shadow-sm text-gray-700 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-blue-500"
      />

      <button
        type="button"
        onClick={async () => {
          if (mfaCode) {
            console.log("MFA Auth App Code:", mfaCode);
            // Call the function to confirm the MFA secret here
            try {
              await confirmMfaUserSecret({ mfaAuthAppCode: mfaCode });
              alert("MFA Secret Confirmed!");
            } catch (error) {
              alert("MFA Auth App Code is required.");
              console.error("Error confirming MFA secret:", error);
            }
          } else {
            console.error("MFA Auth App Code is required.");

            alert("MFA Auth App Code is required.");
          }
        }}
        className="mt-4 px-4 py-2 text-white bg-green-500 rounded-md shadow-md hover:bg-green-600 focus:outline-none focus:ring-2 focus:ring-green-500 focus:ring-offset-2 focus:ring-offset-white"
      >
        Confirm MFA Secret
      </button>
    </div>
  );
}
