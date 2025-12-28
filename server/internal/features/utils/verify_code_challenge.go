package application_utils

import (
	"crypto/rand"
	"crypto/sha256"
	"encoding/base64"
)

func VerifyCodeChallenge(codeVerifier, codeChallenge, codeChallengeMethod string) (bool, error) {
	generatedCodeChallenge := GenerateCodeChallenge(codeVerifier, codeChallengeMethod)

	return generatedCodeChallenge == codeChallenge, nil
}

// GenerateCodeChallenge gera o code_challenge a partir do code_verifier
// seguindo o padrão PKCE (S256).
func GenerateCodeChallenge(codeVerifier string, codeChallengeMethod string) string {
	var hash []byte

	switch codeChallengeMethod {
	case "S256":
		// SHA-256 hash do code_verifier
		sum := sha256.Sum256([]byte(codeVerifier))
		hash = sum[:]
	case "plain":
		// No método "plain", o code_challenge é igual ao code_verifier
		return codeVerifier
	default:
		// Método desconhecido
		return ""
	}

	// Base64 URL-safe sem padding (equivalente ao replace do JS)
	return base64.RawURLEncoding.EncodeToString(hash[:])
}

func GenerateCodeVerifier() (string, error) {
	b := make([]byte, 32) // 32 bytes → ~43 chars
	if _, err := rand.Read(b); err != nil {
		return "", err
	}
	return base64.RawURLEncoding.EncodeToString(b), nil
}
