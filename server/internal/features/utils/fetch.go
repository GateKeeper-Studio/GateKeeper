package application_utils

import (
	"bytes"
	"encoding/json"
	"io"
	"net/http"
	"net/url"
	"time"
)

// FetchOptions allows for the configuration of an HTTP request.
// It's used as an argument to the Fetch function.
type FetchOptions struct {
	// Body is the request body. It will be JSON-encoded if it's not nil.
	Body interface{}
	// Params represents the URL query parameters.
	Params url.Values
	// Headers represents the custom HTTP headers to be sent.
	Headers map[string]string
}

// Fetch is a utility function that simplifies making HTTP requests, similar to the Fetch API in JavaScript.
// It returns the http.Response, which the caller is responsible for closing.
func Fetch(method, urlStr string, options *FetchOptions) (*http.Response, error) {
	// Use a client with a timeout to prevent requests from hanging indefinitely.
	client := &http.Client{Timeout: 15 * time.Second}

	var bodyReader io.Reader
	isJSON := false

	// If options are provided, process them.
	if options != nil {
		// 1. Handle the request body if it exists.
		if options.Body != nil {
			// Marshal the Body into JSON.
			jsonData, err := json.Marshal(options.Body)
			if err != nil {
				return nil, err // Return error if JSON marshaling fails.
			}
			bodyReader = bytes.NewBuffer(jsonData)
			isJSON = true
		}

		// 2. Handle URL query parameters.
		if options.Params != nil {
			// Parse the base URL.
			parsedURL, err := url.Parse(urlStr)
			if err != nil {
				return nil, err
			}
			// Encode and attach the query parameters.
			parsedURL.RawQuery = options.Params.Encode()
			urlStr = parsedURL.String()
		}
	}

	// Create the request.
	req, err := http.NewRequest(method, urlStr, bodyReader)
	if err != nil {
		return nil, err // Return error if request creation fails.
	}

	// 3. Set headers.
	if isJSON {
		// Set the content type to JSON if a body was provided.
		req.Header.Set("Content-Type", "application/json; charset=UTF-8")
	}

	// Add any custom headers from options.
	// This will override the default "Content-Type" if it's also set in options.Headers.
	if options != nil && options.Headers != nil {
		for key, value := range options.Headers {
			req.Header.Set(key, value)
		}
	}

	// Execute the request.
	return client.Do(req)
}
