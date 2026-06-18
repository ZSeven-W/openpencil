/* tslint:disable */
/* eslint-disable */

/**
 * Long-lived shell handle. The smoke HTML must keep this alive (e.g.
 * `window.__opShell = mount("op")`) so closures stored on the shell
 * remain reachable for the page lifetime.
 *
 * The stub variant (without `skia` feature) carries no fields and exists
 * only so the wasm32-unknown-unknown CI baseline can compile-check the
 * public surface.
 */
export class WebShell {
    private constructor();
    free(): void;
    [Symbol.dispose](): void;
    /**
     * Re-read the host canvas backing-store and CSS size, rebuild the Skia
     * surface when needed, then repaint in logical CSS-pixel coordinates.
     */
    resize(): void;
}

/**
 * Mount the WebShell on the canvas identified by `canvas_id` in the host
 * document. Returns the live shell instance to the caller; the caller
 * MUST keep it alive (`window.__opShell = mount("op")`).
 *
 * Errors propagate back to JS as a `JsValue` exception.
 *
 * Without the `skia` feature this is a stub that returns the
 * fields-less `WebShell` after validating the canvas element exists
 * — useful only for the kickoff §1.2 wasm32-clean compile guard CI.
 */
export function mount(canvas_id: string): WebShell;

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly __wbg_webshell_free: (a: number, b: number) => void;
    readonly mount: (a: number, b: number) => [number, number, number];
    readonly webshell_resize: (a: number) => [number, number];
    readonly emscripten_glActiveTexture: (a: number) => void;
    readonly emscripten_glAttachShader: (a: number, b: number) => void;
    readonly emscripten_glBeginQuery: (a: number, b: number) => void;
    readonly emscripten_glBeginQueryEXT: (a: number, b: number) => void;
    readonly emscripten_glBindAttribLocation: (a: number, b: number, c: number) => void;
    readonly emscripten_glBindBuffer: (a: number, b: number) => void;
    readonly emscripten_glBindFramebuffer: (a: number, b: number) => void;
    readonly emscripten_glBindVertexArray: (a: number) => void;
    readonly emscripten_glBindRenderbuffer: (a: number, b: number) => void;
    readonly emscripten_glBindSampler: (a: number, b: number) => void;
    readonly emscripten_glBindTexture: (a: number, b: number) => void;
    readonly emscripten_glBindVertexArrayOES: (a: number) => void;
    readonly emscripten_glBlendColor: (a: number, b: number, c: number, d: number) => void;
    readonly emscripten_glBlendEquation: (a: number) => void;
    readonly emscripten_glBlendFunc: (a: number, b: number) => void;
    readonly emscripten_glBlitFramebuffer: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number, i: number, j: number) => void;
    readonly emscripten_glBufferData: (a: number, b: number, c: number, d: number) => void;
    readonly emscripten_glBufferSubData: (a: number, b: number, c: number, d: number) => void;
    readonly emscripten_glCheckFramebufferStatus: (a: number) => number;
    readonly emscripten_glClear: (a: number) => void;
    readonly emscripten_glClearColor: (a: number, b: number, c: number, d: number) => void;
    readonly emscripten_glClearStencil: (a: number) => void;
    readonly emscripten_glClientWaitSync: (a: number, b: number, c: bigint) => number;
    readonly emscripten_glColorMask: (a: number, b: number, c: number, d: number) => void;
    readonly emscripten_glCompileShader: (a: number) => void;
    readonly emscripten_glCompressedTexImage2D: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number) => void;
    readonly emscripten_glCompressedTexSubImage2D: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number, i: number) => void;
    readonly emscripten_glCopyBufferSubData: (a: number, b: number, c: number, d: number, e: number) => void;
    readonly emscripten_glCopyTexSubImage2D: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number) => void;
    readonly emscripten_glCreateProgram: () => number;
    readonly emscripten_glCreateShader: (a: number) => number;
    readonly emscripten_glCullFace: (a: number) => void;
    readonly emscripten_glDeleteBuffers: (a: number, b: number) => void;
    readonly emscripten_glDeleteFramebuffers: (a: number, b: number) => void;
    readonly emscripten_glDeleteProgram: (a: number) => void;
    readonly emscripten_glDeleteQueries: (a: number, b: number) => void;
    readonly emscripten_glDeleteQueriesEXT: (a: number, b: number) => void;
    readonly emscripten_glDeleteRenderbuffers: (a: number, b: number) => void;
    readonly emscripten_glDeleteSamplers: (a: number, b: number) => void;
    readonly emscripten_glDeleteShader: (a: number) => void;
    readonly emscripten_glDeleteSync: (a: number) => void;
    readonly emscripten_glDeleteTextures: (a: number, b: number) => void;
    readonly emscripten_glDeleteVertexArrays: (a: number, b: number) => void;
    readonly emscripten_glDeleteVertexArraysOES: (a: number, b: number) => void;
    readonly emscripten_glDepthMask: (a: number) => void;
    readonly emscripten_glDisable: (a: number) => void;
    readonly emscripten_glDisableVertexAttribArray: (a: number) => void;
    readonly emscripten_glDrawArrays: (a: number, b: number, c: number) => void;
    readonly emscripten_glDrawArraysInstanced: (a: number, b: number, c: number, d: number) => void;
    readonly emscripten_glDrawArraysInstancedBaseInstanceWEBGL: (a: number, b: number, c: number, d: number, e: number) => void;
    readonly emscripten_glDrawBuffers: (a: number, b: number) => void;
    readonly emscripten_glDrawElements: (a: number, b: number, c: number, d: number) => void;
    readonly emscripten_glDrawElementsInstanced: (a: number, b: number, c: number, d: number, e: number) => void;
    readonly emscripten_glDrawElementsInstancedBaseVertexBaseInstanceWEBGL: (a: number, b: number, c: number, d: number, e: number, f: number, g: number) => void;
    readonly emscripten_glDrawRangeElements: (a: number, b: number, c: number, d: number, e: number, f: number) => void;
    readonly emscripten_glEnable: (a: number) => void;
    readonly emscripten_glEnableVertexAttribArray: (a: number) => void;
    readonly emscripten_glEndQueryEXT: (a: number) => void;
    readonly emscripten_glFenceSync: (a: number, b: number) => number;
    readonly emscripten_glFinish: () => void;
    readonly emscripten_glFlush: () => void;
    readonly emscripten_glFramebufferRenderbuffer: (a: number, b: number, c: number, d: number) => void;
    readonly emscripten_glFramebufferTexture2D: (a: number, b: number, c: number, d: number, e: number) => void;
    readonly emscripten_glFrontFace: (a: number) => void;
    readonly emscripten_glGenBuffers: (a: number, b: number) => void;
    readonly emscripten_glGenFramebuffers: (a: number, b: number) => void;
    readonly emscripten_glGenQueries: (a: number, b: number) => void;
    readonly emscripten_glGenQueriesEXT: (a: number, b: number) => void;
    readonly emscripten_glGenSamplers: (a: number, b: number) => void;
    readonly emscripten_glGenTextures: (a: number, b: number) => void;
    readonly emscripten_glGenVertexArrays: (a: number, b: number) => void;
    readonly emscripten_glGenVertexArraysOES: (a: number, b: number) => void;
    readonly emscripten_glGetBufferParameteriv: (a: number, b: number, c: number) => void;
    readonly emscripten_glGetError: () => number;
    readonly emscripten_glGetFloatv: (a: number, b: number) => void;
    readonly emscripten_glGetProgramInfoLog: (a: number, b: number, c: number, d: number) => void;
    readonly emscripten_glGetProgramiv: (a: number, b: number, c: number) => void;
    readonly emscripten_glGetQueryObjecti64vEXT: (a: number, b: number, c: number) => void;
    readonly emscripten_glGetQueryObjectui64vEXT: (a: number, b: number, c: number) => void;
    readonly emscripten_glGetQueryivEXT: (a: number, b: number, c: number) => void;
    readonly emscripten_glRenderbufferStorage: (a: number, b: number, c: number, d: number) => void;
    readonly emscripten_glScissor: (a: number, b: number, c: number, d: number) => void;
    readonly emscripten_glTexParameterf: (a: number, b: number, c: number) => void;
    readonly emscripten_glTexParameteri: (a: number, b: number, c: number) => void;
    readonly emscripten_glTexSubImage2D: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number, i: number) => void;
    readonly emscripten_glEndQuery: (a: number) => void;
    readonly emscripten_glGenRenderbuffers: (a: number, b: number) => void;
    readonly emscripten_glGenerateMipmap: (a: number) => void;
    readonly emscripten_glGetShaderPrecisionFormat: (a: number, b: number, c: number, d: number) => void;
    readonly emscripten_glGetString: (a: number) => number;
    readonly emscripten_glGetUniformLocation: (a: number, b: number) => number;
    readonly emscripten_glQueryCounterEXT: (a: number, b: number) => void;
    readonly emscripten_glGetFramebufferAttachmentParameteriv: (a: number, b: number, c: number, d: number) => void;
    readonly emscripten_glGetQueryObjectuiv: (a: number, b: number, c: number) => void;
    readonly emscripten_glGetIntegerv: (a: number, b: number) => void;
    readonly emscripten_glGetQueryObjectuivEXT: (a: number, b: number, c: number) => void;
    readonly emscripten_glGetQueryiv: (a: number, b: number, c: number) => void;
    readonly emscripten_glGetRenderbufferParameteriv: (a: number, b: number, c: number) => void;
    readonly emscripten_glGetShaderInfoLog: (a: number, b: number, c: number, d: number) => void;
    readonly emscripten_glGetShaderiv: (a: number, b: number, c: number) => void;
    readonly emscripten_glGetStringi: (a: number, b: number) => number;
    readonly emscripten_glInvalidateFramebuffer: (a: number, b: number, c: number) => void;
    readonly emscripten_glInvalidateSubFramebuffer: (a: number, b: number, c: number, d: number, e: number, f: number, g: number) => void;
    readonly emscripten_glIsSync: (a: number) => number;
    readonly emscripten_glIsTexture: (a: number) => number;
    readonly emscripten_glLineWidth: (a: number) => void;
    readonly emscripten_glLinkProgram: (a: number) => void;
    readonly emscripten_glMultiDrawArraysInstancedBaseInstanceWEBGL: (a: number, b: number, c: number, d: number, e: number, f: number) => void;
    readonly emscripten_glMultiDrawElementsInstancedBaseVertexBaseInstanceWEBGL: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number) => void;
    readonly emscripten_glPixelStorei: (a: number, b: number) => void;
    readonly emscripten_glReadBuffer: (a: number) => void;
    readonly emscripten_glReadPixels: (a: number, b: number, c: number, d: number, e: number, f: number, g: number) => void;
    readonly emscripten_glRenderbufferStorageMultisample: (a: number, b: number, c: number, d: number, e: number) => void;
    readonly emscripten_glSamplerParameterf: (a: number, b: number, c: number) => void;
    readonly emscripten_glSamplerParameteri: (a: number, b: number, c: number) => void;
    readonly emscripten_glSamplerParameteriv: (a: number, b: number, c: number) => void;
    readonly emscripten_glShaderSource: (a: number, b: number, c: number, d: number) => void;
    readonly emscripten_glStencilFunc: (a: number, b: number, c: number) => void;
    readonly emscripten_glStencilFuncSeparate: (a: number, b: number, c: number, d: number) => void;
    readonly emscripten_glStencilMask: (a: number) => void;
    readonly emscripten_glStencilMaskSeparate: (a: number, b: number) => void;
    readonly emscripten_glStencilOp: (a: number, b: number, c: number) => void;
    readonly emscripten_glStencilOpSeparate: (a: number, b: number, c: number, d: number) => void;
    readonly emscripten_glTexImage2D: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number, i: number) => void;
    readonly emscripten_glTexParameterfv: (a: number, b: number, c: number) => void;
    readonly emscripten_glTexParameteriv: (a: number, b: number, c: number) => void;
    readonly emscripten_glTexStorage2D: (a: number, b: number, c: number, d: number, e: number) => void;
    readonly emscripten_glUniform1f: (a: number, b: number) => void;
    readonly emscripten_glUniform1fv: (a: number, b: number, c: number) => void;
    readonly emscripten_glUniform1i: (a: number, b: number) => void;
    readonly emscripten_glUniform1iv: (a: number, b: number, c: number) => void;
    readonly emscripten_glUniform2f: (a: number, b: number, c: number) => void;
    readonly emscripten_glUniform2fv: (a: number, b: number, c: number) => void;
    readonly emscripten_glUniform2i: (a: number, b: number, c: number) => void;
    readonly emscripten_glUniform2iv: (a: number, b: number, c: number) => void;
    readonly emscripten_glUniform3f: (a: number, b: number, c: number, d: number) => void;
    readonly emscripten_glUniform3fv: (a: number, b: number, c: number) => void;
    readonly emscripten_glUniform3i: (a: number, b: number, c: number, d: number) => void;
    readonly emscripten_glUniform3iv: (a: number, b: number, c: number) => void;
    readonly emscripten_glUniform4f: (a: number, b: number, c: number, d: number, e: number) => void;
    readonly emscripten_glUniform4fv: (a: number, b: number, c: number) => void;
    readonly emscripten_glUniform4i: (a: number, b: number, c: number, d: number, e: number) => void;
    readonly emscripten_glUniform4iv: (a: number, b: number, c: number) => void;
    readonly emscripten_glUniformMatrix2fv: (a: number, b: number, c: number, d: number) => void;
    readonly emscripten_glUniformMatrix3fv: (a: number, b: number, c: number, d: number) => void;
    readonly emscripten_glUniformMatrix4fv: (a: number, b: number, c: number, d: number) => void;
    readonly emscripten_glUseProgram: (a: number) => void;
    readonly emscripten_glVertexAttrib1f: (a: number, b: number) => void;
    readonly emscripten_glVertexAttrib2fv: (a: number, b: number) => void;
    readonly emscripten_glVertexAttrib3fv: (a: number, b: number) => void;
    readonly emscripten_glVertexAttrib4fv: (a: number, b: number) => void;
    readonly emscripten_glVertexAttribDivisor: (a: number, b: number) => void;
    readonly emscripten_glVertexAttribIPointer: (a: number, b: number, c: number, d: number, e: number) => void;
    readonly emscripten_glVertexAttribPointer: (a: number, b: number, c: number, d: number, e: number, f: number) => void;
    readonly emscripten_glViewport: (a: number, b: number, c: number, d: number) => void;
    readonly emscripten_glWaitSync: (a: number, b: number, c: bigint) => void;
    readonly glGetIntegerv: (a: number, b: number) => void;
    readonly glGetString: (a: number) => number;
    readonly glGetStringi: (a: number, b: number) => number;
    readonly _Znwm: (a: number) => number;
    readonly _ZdlPvm: (a: number, b: number) => void;
    readonly _ZdaPvm: (a: number, b: number) => void;
    readonly abort: () => void;
    readonly _Znam: (a: number) => number;
    readonly strncmp: (a: number, b: number, c: number) => number;
    readonly _ZNSt3__26localeC1Ev: () => void;
    readonly _ZNSt3__26localeD1Ev: () => void;
    readonly __cxa_pure_virtual: () => void;
    readonly _ZdaPv: (a: number) => void;
    readonly __cxa_guard_acquire: (a: number) => number;
    readonly __cxa_atexit: (a: number, b: number, c: number) => number;
    readonly __cxa_guard_release: (a: number) => void;
    readonly fopen: (a: number, b: number) => number;
    readonly fread: (a: number, b: number, c: number, d: number) => number;
    readonly fclose: (a: number) => number;
    readonly calloc: (a: number, b: number) => number;
    readonly malloc: (a: number) => number;
    readonly free: (a: number) => void;
    readonly realloc: (a: number, b: number) => number;
    readonly getenv: (a: number) => number;
    readonly strcmp: (a: number, b: number) => number;
    readonly strncpy: (a: number, b: number, c: number) => number;
    readonly __errno_location: () => number;
    readonly strtol: (a: number, b: number, c: number) => number;
    readonly strstr: (a: number, b: number) => number;
    readonly fputc: (a: number, b: number) => number;
    readonly strcpy: (a: number, b: number) => number;
    readonly strrchr: (a: number, b: number) => number;
    readonly strcat: (a: number, b: number) => number;
    readonly memchr: (a: number, b: number, c: number) => number;
    readonly mmap: (a: number, b: number, c: number, d: number, e: number, f: bigint) => number;
    readonly munmap: (a: number, b: number) => number;
    readonly malloc_usable_size: (a: number) => number;
    readonly sem_destroy: (a: number) => number;
    readonly sem_init: (a: number, b: number, c: number) => number;
    readonly sem_post: (a: number) => number;
    readonly sem_wait: (a: number) => number;
    readonly _ZdlPv: (a: number) => void;
    readonly fileno: (a: number) => number;
    readonly fstat: (a: number, b: number) => number;
    readonly pread: (a: number, b: number, c: number, d: bigint) => number;
    readonly ftell: (a: number) => number;
    readonly fseek: (a: number, b: number, c: number) => number;
    readonly nextafterf: (a: number, b: number) => number;
    readonly strtoull: (a: number, b: number, c: number) => bigint;
    readonly _ZNSt3__26locale7classicEv: () => void;
    readonly remainder: (a: number, b: number) => number;
    readonly asinh: (a: number) => number;
    readonly acosh: (a: number) => number;
    readonly atanh: (a: number) => number;
    readonly wmemchr: (a: number, b: number, c: number) => number;
    readonly _ZNSt3__214basic_iostreamIcNS_11char_traitsIcEEED2Ev: () => void;
    readonly _ZNSt3__29basic_iosIcNS_11char_traitsIcEEED2Ev: () => void;
    readonly _ZNSt3__28ios_base4initEPv: () => void;
    readonly _ZNSt3__213basic_istreamIcNS_11char_traitsIcEEE6sentryC1ERS3_b: () => void;
    readonly _ZNSt3__28ios_base5clearEj: () => void;
    readonly longjmp: (a: number, b: number) => void;
    readonly setjmp: (a: number) => number;
    readonly tolower: (a: number) => number;
    readonly _ZnwmRKSt9nothrow_t: (a: number, b: number) => number;
    readonly qsort: (a: number, b: number, c: number, d: number) => void;
    readonly _ZNKSt3__28ios_base6getlocEv: () => void;
    readonly _ZNSt3__213basic_istreamIcNS_11char_traitsIcEEErsERd: () => void;
    readonly _ZNSt3__213basic_istreamIcNS_11char_traitsIcEEErsERf: () => void;
    readonly _ZNSt3__213basic_ostreamIcNS_11char_traitsIcEEElsEf: () => void;
    readonly _ZNSt3__26localeC1ERKS0_: () => void;
    readonly _ZNSt3__26localeaSERKS0_: () => void;
    readonly _ZNSt3__28ios_base5imbueERKNS_6localeE: () => void;
    readonly wasm_libc_shim_stdio_panic: (a: number) => void;
    readonly wasm_bindgen__convert__closures_____invoke__h702e40c9d89ede44: (a: number, b: number, c: any) => void;
    readonly wasm_bindgen__convert__closures_____invoke__hd4940eac09071962: (a: number, b: number) => void;
    readonly __wbindgen_malloc_command_export: (a: number, b: number) => number;
    readonly __wbindgen_realloc_command_export: (a: number, b: number, c: number, d: number) => number;
    readonly __wbindgen_exn_store_command_export: (a: number) => void;
    readonly __externref_table_alloc_command_export: () => number;
    readonly __wbindgen_externrefs: WebAssembly.Table;
    readonly __wbindgen_free_command_export: (a: number, b: number, c: number) => void;
    readonly __wbindgen_destroy_closure_command_export: (a: number, b: number) => void;
    readonly __externref_table_dealloc_command_export: (a: number) => void;
    readonly __wbindgen_start: () => void;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;

/**
 * Instantiates the given `module`, which can either be bytes or
 * a precompiled `WebAssembly.Module`.
 *
 * @param {{ module: SyncInitInput }} module - Passing `SyncInitInput` directly is deprecated.
 *
 * @returns {InitOutput}
 */
export function initSync(module: { module: SyncInitInput } | SyncInitInput): InitOutput;

/**
 * If `module_or_path` is {RequestInfo} or {URL}, makes a request and
 * for everything else, calls `WebAssembly.instantiate` directly.
 *
 * @param {{ module_or_path: InitInput | Promise<InitInput> }} module_or_path - Passing `InitInput` directly is deprecated.
 *
 * @returns {Promise<InitOutput>}
 */
export default function __wbg_init (module_or_path?: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>): Promise<InitOutput>;
