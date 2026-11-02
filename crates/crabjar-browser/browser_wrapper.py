#!/usr/bin/env python3
"""
crabjar-browser Playwright wrapper.

Provides a JSON-RPC-like interface over stdin/stdout for browser automation.
Commands are received as JSON objects on stdin, responses are sent as JSON
objects on stdout.

Protocol:
  Request: {"id": <int>, "method": "<string>", "params": {...}}
  Response: {"id": <int>, "result": {...}} or {"id": <int>, "error": "..."}
"""

import json
import sys
import asyncio
from playwright.async_api import async_playwright, TimeoutError as PlaywrightTimeout


class BrowserSession:
    def __init__(self):
        self.pw = None
        self.browser = None
        self.context = None
        self.page = None
        self.request_id = 0

    async def start(self):
        self.pw = await async_playwright().start()
        self.browser = await self.pw.chromium.launch(
            headless=True,
            args=[
                "--no-sandbox",
                "--disable-dev-shm-usage",
                "--disable-gpu",
            ]
        )
        self.context = await self.browser.new_context(
            viewport={"width": 1280, "height": 720},
            user_agent="Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36"
        )
        self.page = await self.context.new_page()
        assert self.page is not None, "Failed to create page"
        return {"status": "started"}

    def _require_page(self):
        if self.page is None:
            raise RuntimeError("Browser not started or page not created")
        return self.page

    async def navigate(self, url, timeout=30000):
        page = self._require_page()
        try:
            response = await page.goto(url, timeout=timeout, wait_until="networkidle")
            return {
                "status": "success",
                "url": page.url,
                "title": await page.title(),
                "status_code": response.status if response else None
            }
        except PlaywrightTimeout:
            return {"status": "timeout", "url": url}
        except Exception as e:
            return {"status": "error", "message": str(e)}

    async def evaluate(self, expression):
        page = self._require_page()
        try:
            result = await page.evaluate(expression)
            return {"status": "success", "result": result}
        except Exception as e:
            return {"status": "error", "message": str(e)}

    async def fill(self, selector, value):
        page = self._require_page()
        try:
            await page.fill(selector, value)
            return {"status": "success"}
        except Exception as e:
            return {"status": "error", "message": str(e)}

    async def click(self, selector):
        page = self._require_page()
        try:
            await page.click(selector)
            return {"status": "success"}
        except Exception as e:
            return {"status": "error", "message": str(e)}

    async def screenshot(self, path=None):
        page = self._require_page()
        try:
            if path:
                await page.screenshot(path=path)
                return {"status": "success", "path": path}
            else:
                data = await page.screenshot()
                # Return base64 encoded
                import base64
                return {"status": "success", "data": base64.b64encode(data).decode()}
        except Exception as e:
            return {"status": "error", "message": str(e)}

    async def content(self):
        page = self._require_page()
        try:
            html = await page.content()
            return {"status": "success", "html": html}
        except Exception as e:
            return {"status": "error", "message": str(e)}

    async def text_content(self, selector=None):
        page = self._require_page()
        try:
            if selector:
                element = await page.query_selector(selector)
                if element:
                    text = await element.text_content()
                    return {"status": "success", "text": text}
                return {"status": "error", "message": "Element not found"}
            else:
                text = await page.text_content("body")
                return {"status": "success", "text": text}
        except Exception as e:
            return {"status": "error", "message": str(e)}

    async def close(self):
        try:
            if self.page is not None:
                await self.page.close()
            if self.context is not None:
                await self.context.close()
            if self.browser is not None:
                await self.browser.close()
            if self.pw is not None:
                await self.pw.stop()
            return {"status": "closed"}
        except Exception as e:
            return {"status": "error", "message": str(e)}


async def handle_request(session, request):
    method = request.get("method")
    params = request.get("params", {})

    if method == "start":
        return await session.start()
    elif method == "navigate":
        return await session.navigate(params["url"], params.get("timeout", 30000))
    elif method == "evaluate":
        return await session.evaluate(params["expression"])
    elif method == "fill":
        return await session.fill(params["selector"], params["value"])
    elif method == "click":
        return await session.click(params["selector"])
    elif method == "screenshot":
        return await session.screenshot(params.get("path"))
    elif method == "content":
        return await session.content()
    elif method == "text_content":
        return await session.text_content(params.get("selector"))
    elif method == "close":
        return await session.close()
    else:
        return {"status": "error", "message": f"Unknown method: {method}"}


async def main():
    session = BrowserSession()

    for line in sys.stdin:
        line = line.strip()
        if not line:
            continue

        try:
            request = json.loads(line)
        except json.JSONDecodeError as e:
            response = {"id": None, "error": f"Invalid JSON: {e}"}
            print(json.dumps(response), flush=True)
            continue

        request_id = request.get("id")
        try:
            result = await handle_request(session, request)
            response = {"id": request_id, "result": result}
        except Exception as e:
            response = {"id": request_id, "error": str(e)}

        print(json.dumps(response), flush=True)


if __name__ == "__main__":
    asyncio.run(main())