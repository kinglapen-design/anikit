#!/usr/bin/env python3
"""AniKit CEP spike —— 最小 WS server（模拟 AniKit 侧通信服务）。

行为：绑定 127.0.0.1 动态端口 → 写端口文件 %APPDATA%/AniKit/cep-server.port（CEP 面板经 node fs 读它发现端口）
→ 打印收到的每条消息并回 echo。Ctrl+C 退出时删除端口文件。

运行：tools/.venv/Scripts/python.exe spike/cep-proto/ws_server.py
"""

import asyncio
import json
import os
import sys

PORT_DIR = os.path.join(os.environ.get("APPDATA", ""), "AniKit")
PORT_FILE = os.path.join(PORT_DIR, "cep-server.port")


async def handle(ws):
    print(f"[+] 客户端接入: {ws.remote_address}", flush=True)
    try:
        async for msg in ws:
            print(f"[<] {msg}", flush=True)
            try:
                data = json.loads(msg)
                await ws.send(json.dumps({"type": "echo", "data": data}, ensure_ascii=False))
            except json.JSONDecodeError:
                await ws.send(json.dumps({"type": "pong"}))
    finally:
        print(f"[-] 客户端断开: {ws.remote_address}", flush=True)


async def main():
    import websockets

    os.makedirs(PORT_DIR, exist_ok=True)
    async with websockets.serve(handle, "127.0.0.1", 0) as server:
        port = server.sockets[0].getsockname()[1]
        with open(PORT_FILE, "w", encoding="utf-8") as f:
            f.write(str(port))
        print(f"WS server 就绪: ws://127.0.0.1:{port}", flush=True)
        print(f"端口文件: {PORT_FILE}", flush=True)
        try:
            await asyncio.Future()  # 永久运行直到 Ctrl+C
        finally:
            try:
                os.remove(PORT_FILE)
            except OSError:
                pass


if __name__ == "__main__":
    try:
        asyncio.run(main())
    except KeyboardInterrupt:
        sys.exit(0)
