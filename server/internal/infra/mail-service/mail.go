package mailservice

import (
	"context"
	"io"
	"log/slog"
	"os"
	"strconv"
	"strings"

	"github.com/google/uuid"
	gomail "gopkg.in/mail.v2"
)

type IMailService interface {
	SendEmailConfirmationEmail(ctx context.Context, to, userName, token string) error
	SendMfaEmail(ctx context.Context, to, userName, token string) error
	SendForgotPasswordEmail(ctx context.Context, to, userName, token string, passwordResetID, applicationID uuid.UUID) error
}

type MailService struct{}

type SendMailParams struct {
	To      string
	Subject string
	Body    string
}

func (ms *MailService) sendMail(ctx context.Context, params SendMailParams) error {
	message := gomail.NewMessage()

	message.SetHeader("From", os.Getenv("MAIL_USERNAME"))
	message.SetHeader("To", params.To)
	message.SetHeader("Subject", params.Subject)

	// Set email body
	message.SetBody("text/html", params.Body)

	mailUsername := os.Getenv("MAIL_USERNAME")
	mailPassword := os.Getenv("MAIL_PASSWORD")
	mailHost := os.Getenv("MAIL_HOST")
	mailPort := os.Getenv("MAIL_PORT")

	mailPortInt, err := strconv.Atoi(mailPort)

	if err != nil {
		return err
	}

	dialer := gomail.NewDialer(mailHost, mailPortInt, mailUsername, mailPassword)

	if err := dialer.DialAndSend(message); err != nil {
		slog.Error("Error sending email: %v", err.Error(), err)

		panic(err)
	}

	return nil
}

func (ms *MailService) SendEmailConfirmationEmail(ctx context.Context, to, userName, token string) error {
	emailConfirmationTemplate, err := readFileAsString("./internal/infra/mail-service/email-confirmation-template.html")

	if err != nil {
		return err
	}

	replacedString := strings.Replace(emailConfirmationTemplate, "{{$name}}", userName, -1)
	replacedString = strings.Replace(replacedString, "{{$confirmation-token}}", token, -1)

	ms.sendMail(ctx, SendMailParams{
		To:      to,
		Subject: "E-mail Confirmation",
		Body:    replacedString,
	})

	return nil
}

func (ms *MailService) SendForgotPasswordEmail(ctx context.Context, to, userName, token string, passwordResetID, applicationID uuid.UUID) error {
	emailConfirmationTemplate, err := readFileAsString("./internal/infra/mail-service/forgot-password-template.html")

	if err != nil {
		return err
	}

	url := os.Getenv("CLIENT_APPLICATION_URL") + "/auth/" + applicationID.String() + "/change-password?token=" + token + "&id=" + passwordResetID.String()

	replacedString := strings.Replace(emailConfirmationTemplate, "{{$name}}", userName, -1)
	replacedString = strings.Replace(replacedString, "{{$confirmation-url}}", url, -1)

	ms.sendMail(ctx, SendMailParams{
		To:      to,
		Subject: "E-mail Confirmation",
		Body:    replacedString,
	})

	return nil
}

func (ms *MailService) SendMfaEmail(ctx context.Context, to, userName, token string) error {
	emailConfirmationTemplate, err := readFileAsString("./internal/infra/mail-service/mfa-email-template.html")

	if err != nil {
		return err
	}

	replacedString := strings.Replace(emailConfirmationTemplate, "{{$name}}", userName, -1)
	replacedString = strings.Replace(replacedString, "{{$confirmation-token}}", token, -1)

	ms.sendMail(ctx, SendMailParams{
		To:      to,
		Subject: "Multi Factor Authentication",
		Body:    replacedString,
	})

	return nil
}

func readFileAsString(filePath string) (string, error) {
	// Open the file
	file, err := os.Open(filePath)
	if err != nil {
		return "", err
	}
	defer file.Close()

	// Read the file content into a string
	content, err := io.ReadAll(file)
	if err != nil {
		return "", err
	}

	return string(content), nil
}
