import { defineConfig } from 'vite';
export default defineConfig({build:{target:'es2022',outDir:'dist'},server:{port:9071,proxy:{'/v1':'http://127.0.0.1:9070','/health':'http://127.0.0.1:9070'}}});
