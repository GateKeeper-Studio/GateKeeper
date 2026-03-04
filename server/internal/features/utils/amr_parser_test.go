package application_utils

import (
	"encoding/base64"
	"encoding/json"
	"testing"
)

func buildIDToken(claims map[string]interface{}) string {
	header := base64.RawURLEncoding.EncodeToString([]byte(`{"alg":"RS256","typ":"JWT"}`))
	payloadBytes, _ := json.Marshal(claims)
	payload := base64.RawURLEncoding.EncodeToString(payloadBytes)
	sig := base64.RawURLEncoding.EncodeToString([]byte("fake-signature"))
	return header + "." + payload + "." + sig
}

func TestExtractAmrFromIDToken_WithAmr(t *testing.T) {
	token := buildIDToken(map[string]interface{}{
		"sub": "1234567890",
		"amr": []string{"pwd", "mfa", "otp"},
		"iat": 1234567890,
	})

	amr := ExtractAmrFromIDToken(token)

	if len(amr) != 3 {
		t.Fatalf("Expected 3 AMR values, got %d: %v", len(amr), amr)
	}

	expected := []string{"pwd", "mfa", "otp"}
	for i, v := range expected {
		if amr[i] != v {
			t.Errorf("AMR[%d] = %q, want %q", i, amr[i], v)
		}
	}
}

func TestExtractAmrFromIDToken_NoAmr(t *testing.T) {
	token := buildIDToken(map[string]interface{}{
		"sub": "1234567890",
		"iat": 1234567890,
	})

	amr := ExtractAmrFromIDToken(token)

	if amr != nil {
		t.Errorf("Expected nil AMR for token without amr claim, got %v", amr)
	}
}

func TestExtractAmrFromIDToken_EmptyString(t *testing.T) {
	amr := ExtractAmrFromIDToken("")

	if amr != nil {
		t.Errorf("Expected nil AMR for empty token, got %v", amr)
	}
}

func TestExtractAmrFromIDToken_InvalidToken(t *testing.T) {
	testCases := []struct {
		name  string
		token string
	}{
		{"not jwt", "not-a-jwt"},
		{"two parts", "a.b"},
		{"invalid base64", "a.!!!.c"},
		{"invalid json", "a." + base64.RawURLEncoding.EncodeToString([]byte("not-json")) + ".c"},
	}

	for _, tc := range testCases {
		t.Run(tc.name, func(t *testing.T) {
			amr := ExtractAmrFromIDToken(tc.token)
			if amr != nil {
				t.Errorf("Expected nil AMR for invalid token %q, got %v", tc.token, amr)
			}
		})
	}
}

func TestExtractAmrFromIDToken_EmptyAmrArray(t *testing.T) {
	token := buildIDToken(map[string]interface{}{
		"sub": "1234567890",
		"amr": []string{},
	})

	amr := ExtractAmrFromIDToken(token)

	if len(amr) != 0 {
		t.Errorf("Expected empty AMR array, got %v", amr)
	}
}

func TestExtractAmrFromIDToken_SingleAmr(t *testing.T) {
	token := buildIDToken(map[string]interface{}{
		"sub": "1234567890",
		"amr": []string{"hwk"},
	})

	amr := ExtractAmrFromIDToken(token)

	if len(amr) != 1 || amr[0] != "hwk" {
		t.Errorf("Expected [\"hwk\"], got %v", amr)
	}
}
