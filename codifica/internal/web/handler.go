// Package web owns the HTML website's routes and presentation.
package web

import (
	"embed"
	"html/template"
	"net/http"
)

//go:embed templates/*.html
var templates embed.FS

func NewHandler() http.Handler {
	index := template.Must(template.ParseFS(templates, "templates/index.html"))
	mux := http.NewServeMux()
	mux.HandleFunc("GET /{$}", func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Content-Type", "text/html; charset=utf-8")
		_ = index.ExecuteTemplate(w, "index.html", nil)
	})
	mux.HandleFunc("GET /healthz", func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Content-Type", "text/plain; charset=utf-8")
		_, _ = w.Write([]byte("ok\n"))
	})
	return mux
}
