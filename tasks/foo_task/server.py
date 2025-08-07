from http.server import BaseHTTPRequestHandler, HTTPServer

class HelloHandler(BaseHTTPRequestHandler):
    def do_GET(self):
        self.send_response(200)
        self.send_header('Content-Type', 'text/plain; charset=utf-8')
        self.end_headers()
        self.wfile.write(b"Hello world")

if __name__ == '__main__':
    port = 3000
    server = HTTPServer(('', port), HelloHandler)
    print(f"Listening on http://0.0.0.0:{port}")
    server.serve_forever()
