package application_utils

import (
	"testing"

	"github.com/stretchr/testify/assert"
	"github.com/stretchr/testify/require"
)

func TestGenerateCodeVerifier_ReturnsNonEmptyString(t *testing.T) {
	verifier, err := GenerateCodeVerifier()
	require.NoError(t, err)
	assert.NotEmpty(t, verifier)
}

func TestGenerateCodeVerifier_IsURLSafe(t *testing.T) {
	for i := 0; i < 10; i++ {
		verifier, err := GenerateCodeVerifier()
		require.NoError(t, err)
		for _, c := range verifier {
			assert.True(t,
				(c >= 'A' && c <= 'Z') || (c >= 'a' && c <= 'z') || (c >= '0' && c <= '9') || c == '-' || c == '_',
				"verifier contains non-URL-safe character: %c", c,
			)
		}
	}
}

func TestGenerateCodeVerifier_IsUnique(t *testing.T) {
	v1, err := GenerateCodeVerifier()
	require.NoError(t, err)
	v2, err := GenerateCodeVerifier()
	require.NoError(t, err)
	assert.NotEqual(t, v1, v2)
}

func TestGenerateCodeChallenge_S256_DiffersFromVerifier(t *testing.T) {
	verifier, err := GenerateCodeVerifier()
	require.NoError(t, err)
	challenge := GenerateCodeChallenge(verifier, "S256")
	assert.NotEmpty(t, challenge)
	assert.NotEqual(t, verifier, challenge)
}

func TestGenerateCodeChallenge_S256_IsDeterministic(t *testing.T) {
	verifier := "fixed_test_verifier_string"
	challenge1 := GenerateCodeChallenge(verifier, "S256")
	challenge2 := GenerateCodeChallenge(verifier, "S256")
	assert.Equal(t, challenge1, challenge2)
}

func TestGenerateCodeChallenge_Plain_EqualsVerifier(t *testing.T) {
	verifier, err := GenerateCodeVerifier()
	require.NoError(t, err)
	challenge := GenerateCodeChallenge(verifier, "plain")
	assert.Equal(t, verifier, challenge)
}

func TestGenerateCodeChallenge_UnknownMethod_ReturnsEmpty(t *testing.T) {
	challenge := GenerateCodeChallenge("anyVerifier", "md5")
	assert.Empty(t, challenge)
}

func TestVerifyCodeChallenge_S256_Valid(t *testing.T) {
	verifier, err := GenerateCodeVerifier()
	require.NoError(t, err)
	challenge := GenerateCodeChallenge(verifier, "S256")

	ok, err := VerifyCodeChallenge(verifier, challenge, "S256")
	require.NoError(t, err)
	assert.True(t, ok)
}

func TestVerifyCodeChallenge_S256_WrongVerifier(t *testing.T) {
	verifier, err := GenerateCodeVerifier()
	require.NoError(t, err)
	challenge := GenerateCodeChallenge(verifier, "S256")

	ok, err := VerifyCodeChallenge("wrong-verifier", challenge, "S256")
	require.NoError(t, err)
	assert.False(t, ok)
}

func TestVerifyCodeChallenge_Plain_Valid(t *testing.T) {
	verifier, err := GenerateCodeVerifier()
	require.NoError(t, err)

	ok, err := VerifyCodeChallenge(verifier, verifier, "plain")
	require.NoError(t, err)
	assert.True(t, ok)
}

func TestVerifyCodeChallenge_Plain_WrongChallenge(t *testing.T) {
	ok, err := VerifyCodeChallenge("verifier-a", "verifier-b", "plain")
	require.NoError(t, err)
	assert.False(t, ok)
}

func TestVerifyCodeChallenge_UnknownMethod_ReturnsFalse(t *testing.T) {
	ok, err := VerifyCodeChallenge("v", "c", "sha512")
	require.NoError(t, err)
	assert.False(t, ok)
}

func TestVerifyCodeChallenge_S256_RoundTrip(t *testing.T) {
	// Verifies the full PKCE flow: generate → challenge → verify
	verifier, err := GenerateCodeVerifier()
	require.NoError(t, err)
	challenge := GenerateCodeChallenge(verifier, "S256")

	ok, err := VerifyCodeChallenge(verifier, challenge, "S256")
	require.NoError(t, err)
	assert.True(t, ok)

	// A different verifier must not match the same challenge
	otherVerifier, err := GenerateCodeVerifier()
	require.NoError(t, err)
	ok, err = VerifyCodeChallenge(otherVerifier, challenge, "S256")
	require.NoError(t, err)
	assert.False(t, ok)
}
