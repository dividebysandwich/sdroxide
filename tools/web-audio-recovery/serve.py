"""Serves the audio-recovery harness and records what it reports.

The page cannot write a file, so every line it wants the runner to see is a
`GET /result?...` and lands here, appended to the runner's output file.

    python serve.py <port> <result-file>
"""

import http.server
import socketserver
import sys
import urllib.parse

PORT = int(sys.argv[1]) if len(sys.argv) > 1 else 8731
OUT = sys.argv[2] if len(sys.argv) > 2 else "result.txt"


class Handler(http.server.SimpleHTTPRequestHandler):
    def do_GET(self):
        url = urllib.parse.urlparse(self.path)
        if url.path == "/result":
            query = urllib.parse.parse_qs(url.query)
            with open(OUT, "a", encoding="utf-8") as f:
                f.write(" ".join(f"{k}={v[0]}" for k, v in query.items()) + "\n")
            self.send_response(204)
            self.end_headers()
            return
        return super().do_GET()

    def log_message(self, *args):
        pass


socketserver.TCPServer.allow_reuse_address = True
with socketserver.TCPServer(("127.0.0.1", PORT), Handler) as httpd:
    httpd.serve_forever()
