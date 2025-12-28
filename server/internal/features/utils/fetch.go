package application_utils

import (
	"bytes"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"net/url"
	"strings"
	"time"
)

// FetchOptions allows for the configuration of an HTTP request.
type FetchOptions struct {
	// Body is the request body. It will be JSON-encoded if Form is nil.
	Body interface{}

	// Form represents application/x-www-form-urlencoded body.
	// If set, it takes precedence over Body.
	Form url.Values

	// Params represents the URL query parameters.
	Params url.Values

	// Headers represents the custom HTTP headers to be sent.
	Headers map[string]string
}

// Fetch is a utility function that simplifies making HTTP requests.
func Fetch(method, urlStr string, options *FetchOptions) (*http.Response, error) {
	client := &http.Client{Timeout: 15 * time.Second}

	var bodyReader io.Reader
	var contentType string

	if options != nil {
		// 1️⃣ Handle form-urlencoded body (highest priority)
		if options.Form != nil {
			encoded := options.Form.Encode()
			bodyReader = strings.NewReader(encoded)
			contentType = "application/x-www-form-urlencoded"

			// 2️⃣ Handle JSON body (default behavior)
		} else if options.Body != nil {
			jsonData, err := json.Marshal(options.Body)
			if err != nil {
				return nil, err
			}
			bodyReader = bytes.NewBuffer(jsonData)
			contentType = "application/json; charset=UTF-8"
		}

		// 3️⃣ Handle URL query parameters
		if options.Params != nil {
			parsedURL, err := url.Parse(urlStr)
			if err != nil {
				return nil, err
			}
			parsedURL.RawQuery = options.Params.Encode()
			urlStr = parsedURL.String()
		}
	}

	req, err := http.NewRequest(method, urlStr, bodyReader)
	if err != nil {
		return nil, err
	}

	// 4️⃣ Set Content-Type automatically (unless overridden)
	if contentType != "" {
		req.Header.Set("Content-Type", contentType)
	}

	// 5️⃣ Apply custom headers (can override Content-Type)
	if options != nil && options.Headers != nil {
		for key, value := range options.Headers {
			req.Header.Set(key, value)
		}
	}

	resp, err := client.Do(req)
	if err != nil {
		return nil, err
	}

	// ✅ 6️⃣ Status code validation
	if resp.StatusCode < 200 || resp.StatusCode >= 300 {
		defer resp.Body.Close()

		bodyBytes, _ := io.ReadAll(resp.Body)
		bodyStr := strings.TrimSpace(string(bodyBytes))

		if bodyStr == "" {
			return nil, fmt.Errorf(
				"http request failed: %s %s -> status %d",
				method,
				urlStr,
				resp.StatusCode,
			)
		}

		return nil, fmt.Errorf(
			"http request failed: %s %s -> status %d, body: %s",
			method,
			urlStr,
			resp.StatusCode,
			bodyStr,
		)
	}

	return resp, nil
}
