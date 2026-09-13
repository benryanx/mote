# Assistant: first generation phase

Available on the development branch, not the published 0.1.0 release. This is
structured pixel drawing by a language model, not an image-diffusion backend.
Art quality depends on the model; meaningful layers are requested but cannot be
guaranteed artistically. AI animation, image uploads and an MCP bridge are not
implemented in this phase.

## Use

1. Open a canvas up to 128×128 and select a nonempty palette and target frame.
2. Click **Assistant** in the header. The window can be moved and resized.
3. Open **⚙ Settings** to select Omarchy agent, or an HTTP provider and its endpoint and model ID.
4. Enter a session-only API key if required, then **Save and close**. Cancel discards edits.
5. Describe the image, approve the context checkbox, and click **Generate preview**.
6. Review the composite preview and choose **Apply as new layers** or discard it.

The prompt is the main view; connection fields live in a separate settings window.
Settings are applied for this session only, not persisted across restarts.
The status and Apply/Discard/Cancel controls stay above the scrolling form.
New previews scroll into view automatically. The prompt starts empty: click
**Use example** to insert actual text. A byte counter shows the 4000-byte limit,
and validation identifies missing model, prompt or consent separately.

Example: “A tiny forest cabin, warm windows, mossy roof, transparent background.
Use separate layers for the cabin and plants, with clear silhouettes and shading.”

The selected frame receives the generated pixels. Other frames receive empty
cels for the new layers. Existing cels, layer locks, frame timing and palette
remain unchanged. Apply is one undo step. No animation is generated.

You can keep editing while the provider runs. Apply requires the original tab
and matching document contents; it refuses a stale preview rather than
overwriting subsequent work. Regenerate after editing, or return to the unchanged
source tab. Closing the Assistant hides it without canceling its request.

## Connections

If Apply is unavailable, use **Go to source** to return to the generating tab.
If that canvas changed or was closed, **Open preview in new tab** recovers the
complete preview (including the original artwork snapshot) without changing
existing tabs. The preview shows its own dimensions separately from the active canvas.

- **Omarchy agent:** reads `omarchy-default-agent`; currently supports **Codex**.
  Requires an installed, signed-in Codex CLI. No API key is entered in Mote.
  Leave Model blank for `gpt-5.6-sol`, or supply an available model ID.
  Agent runs use low reasoning to reduce thinking time; actual speed and model
  access depend on the provider. No live speed benchmark has been run.
  This reuses login, not custom agent configuration: each invocation ignores user
  config and rules, uses an ephemeral session in a private temporary directory,
  requests read-only sandboxing, and disables shell tools, web search and apps.
  These require a CLI version supporting the flags. Managed system policies can
  still apply. Mote does not install agents or change your Omarchy default.
  Your agent provider's usage limits and billing apply; this is not necessarily
  local inference. Cancel terminates the process group Mote launched, but cannot
  guarantee cancellation of remote billing. Output is bounded to 2 MB and the
  generation process to 120 seconds.
- **Ollama:** default `http://127.0.0.1:11434/api/chat`. You must already have a
  running service and an installed model. Mote does not download models, start
  services or automatically choose a model. Requires JSON output support.
- **API:** default `https://api.openai.com/v1/chat/completions`. Enter a model
  available to your API account that supports JSON mode. Other compatible
  endpoints must accept `messages`, `response_format: {"type":"json_object"}`
  and `max_completion_tokens`. Compatibility is not universal. This uses an API
  account, not an existing desktop/chat subscription or agent session.

All settings, prompts and keys are currently session-only and are not written
to the project or workspace preferences. Switching providers clears the key.
Do not put secrets in prompts or URLs. No automatic retries are made.

## Privacy and limits

The request includes your prompt, width, height and indexed RGBA palette, plus
instructions describing Mote's drawing-data format. It does **not** include
existing pixels, filenames, layer names, filesystem paths or other tabs.
Provider retention and billing policies still apply. A local endpoint can itself
use a cloud model; Mote cannot infer whether your service is truly offline.

Only HTTPS and loopback HTTP URLs are accepted, without embedded credentials,
query strings or fragments. Redirects and automatic proxy discovery are disabled.
HTTP requests have a 10-second connection timeout, 120-second overall timeout and
2-MB response limit. At most one request runs at once. For HTTP, Cancel marks the result
for disposal; it does not promise to stop server-side generation or billing.
The next request becomes available when the previous one ends or times out.

Output is strict JSON data, never shell, Python, Rust or JavaScript execution.
The only drawing primitive is a filled rectangle (1×1 for a pixel), grouped into
1–8 layers. Coordinates must fit the original canvas and colors must reference
the supplied palette. Limits are 4096 rectangles, 1,048,576 painted pixel visits,
and the existing document memory/layer limits. Invalid/truncated results do not
modify the project. Empty drawing output is rejected.

## Verification

Tests cover both HTTP adapters against local mock servers, invalid model output,
canvas/palette bounds, preservation of other frames, stale/wrong-tab rejection,
undo/redo, cancel disposal, secret-safe HTTP errors and preview rendering.
No paid API requests were made during development. No running Ollama service
was available for a live model-quality test. End-to-end artistic quality and
real-provider compatibility still need testing with the user's chosen model.
Agent tests cover event parsing, restrictive command flags, stdin transport,
timeouts and cancellation. No live Codex generation was requested during this
implementation; login and model compatibility need a first-run check.

Protocol references:

- [Codex non-interactive mode](https://developers.openai.com/codex/noninteractive)
- [Codex configuration reference](https://developers.openai.com/codex/config-reference)
- [Ollama chat API](https://docs.ollama.com/api/chat)
- [Ollama structured outputs](https://docs.ollama.com/capabilities/structured-outputs)
- [OpenAI Chat Completions](https://developers.openai.com/api/reference/resources/chat/subresources/completions/methods/create)
