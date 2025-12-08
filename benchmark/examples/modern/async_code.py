"""
Async Code Example
Run with: cytoscnpy examples/async_code.py
"""

import asyncio

async def async_helper():
    return "Helper"

async def main_async():
    # Await usage
    result = await async_helper()
    print(result)

    # Async context manager (simulated)
    class AsyncContext:
        async def __aenter__(self):
            return self
        async def __aexit__(self, exc_type, exc, tb):
            pass

    async with AsyncContext() as ctx:
        print("Inside async context")

    # Async iterator (simulated)
    class AsyncIter:
        def __aiter__(self):
            return self
        async def __anext__(self):
            raise StopAsyncIteration

    async for item in AsyncIter():
        print(item)

def main():
    asyncio.run(main_async())

if __name__ == "__main__":
    main()
