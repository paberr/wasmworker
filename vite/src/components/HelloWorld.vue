<script setup lang="ts">
import {
    initWorkerPool,
    WorkerPoolOptions,
    runWorker,
    runPool,
    runParMap,
} from "wasmworker";

let opts = new WorkerPoolOptions();
opts.path = "http://localhost:8000/pkg/wasmworker_demo.js";
initWorkerPool(opts);

defineProps<{ msg: string }>();
</script>

<template>
    <h1>{{ msg }}</h1>

    <input type="number" id="num_values" placeholder="Num Values" />
    <div class="card">
        <button type="button" @click="runWorker">Run Worker</button>
        <button type="button" @click="runPool">Run Pool</button>
        <button type="button" @click="runParMap">Run ParMap</button>
    </div>
    <div id="result">Results go here...</div>
</template>

<style scoped>
.read-the-docs {
    color: #888;
}
</style>
