import asyncio
import sys

import aiohttp


async def _handle_input(websocket):
    while 1:
        to_send = await asyncio.get_running_loop().run_in_executor(None, input)
        await websocket.send_str(to_send)


async def _poll(websocket):
    async for msg in websocket:
        print(msg.data)

        if msg.type in (aiohttp.WSMsgType.CLOSED, aiohttp.WSMsgType.ERROR):
            break


async def main(uri):
    async with aiohttp.ClientSession() as session:
        async with session.ws_connect(uri) as websocket:
            tasks = [
                asyncio.create_task(_handle_input(websocket)),
                asyncio.create_task(_poll(websocket)),
            ]
            done, pending = await asyncio.wait(
                tasks, return_when=asyncio.FIRST_COMPLETED
            )
            for task in done:
                if exc := task.exception():
                    print(exc, file=sys.stderr)

            for task in pending:
                task.cancel()

            print("Disconnected")


asyncio.run(main("ws://localhost:3000/ws/anagram"))
