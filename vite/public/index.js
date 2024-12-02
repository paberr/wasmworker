import init, { runWorker, runPool, runParMap } from '/pkg/wasmworker_demo.js'

init().then(() => {
    console.log('index.js initialized')
    window.runWorker = runWorker
    window.runPool = runPool
    window.runParMap = runParMap
})
