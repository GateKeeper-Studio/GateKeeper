package entities

import (
	"encoding/base64"
	"time"

	"github.com/google/uuid"
)

type MfaWebauthnCredentials struct {
	ID           uuid.UUID
	MfaMethodID  uuid.UUID
	CredentialID string // base64-encoded credential ID bytes
	PublicKey    string // base64-encoded COSE public key bytes
	SignCount    uint32
	CreatedAt    time.Time
}

func NewMfaWebauthnCredentials(mfaMethodID uuid.UUID, credentialID, publicKey []byte, signCount uint32) (*MfaWebauthnCredentials, error) {
	id, err := uuid.NewV7()
	if err != nil {
		return nil, err
	}

	return &MfaWebauthnCredentials{
		ID:           id,
		MfaMethodID:  mfaMethodID,
		CredentialID: base64.StdEncoding.EncodeToString(credentialID),
		PublicKey:    base64.StdEncoding.EncodeToString(publicKey),
		SignCount:    signCount,
		CreatedAt:    time.Now().UTC(),
	}, nil
}

func (c *MfaWebauthnCredentials) CredentialIDBytes() ([]byte, error) {
	return base64.StdEncoding.DecodeString(c.CredentialID)
}

func (c *MfaWebauthnCredentials) PublicKeyBytes() ([]byte, error) {
	return base64.StdEncoding.DecodeString(c.PublicKey)
}
