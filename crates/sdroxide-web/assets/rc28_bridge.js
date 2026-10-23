// The Icom RC-28 remote encoder, over WebHID, for the sdroxide web client.
//
// A knob and three buttons on the operator's own desk, driving a radio that may
// be anywhere — the same thing the native app does through hidraw, SetupAPI or
// IOKit. WebHID exists in Chromium-based browsers (Chrome, Edge, Opera) and
// only in a secure context: https, or http://localhost. Firefox and Safari have
// no WebHID at all, and there `supported()` says so.
//
// This file does no protocol work. It hands raw input reports to the wasm
// client, which parses them with the same code the native app uses
// (`sdroxide_rc28::proto`), and sends whatever reports it is given.
//
// A page may only open a HID device the operator has picked from the browser's
// own chooser, and only in answer to a click — so the first connection is
// `request()`, called from the "Choose device…" chip in Settings → Controls.
// After that the browser remembers the grant, and `setEnabled(true)` finds the
// device again on every later visit and after every replug, with no prompt.
//
// window.sdroxideRc28:
//   supported()        -> bool
//   setEnabled(on)     open (or close) a device already granted to this page
//   request()          show the chooser; must follow a user gesture
//   drain()            -> Array of Uint8Array (a report) or string
//                         ("connected:<name>", "disconnected", "error:<text>")
//   write(Uint8Array)  send an output report (report id 0)
//   connectedName()    -> the open device's name, or "" — the page's own
//                         truth, for every radio tab and not only the one
//                         that happened to drain the "connected" message
//   lastError()        -> the last failure, or ""

(function () {
    const VENDOR = 0x0c26;
    const PRODUCT = 0x001e;
    const FILTERS = [{ vendorId: VENDOR, productId: PRODUCT }];
    // A tab left in the background still receives reports; once nothing is
    // draining them, the oldest go first rather than the page growing without
    // bound.
    const QUEUE_MAX = 512;

    const SUPPORTED = window.isSecureContext && "hid" in navigator;

    let enabled = false;
    let device = null;
    let opening = null;
    let queue = [];
    let lastError = "";
    // Output reports go out one at a time, in order: `sendReport` is a promise
    // and two in flight at once may land in either order.
    let writing = Promise.resolve();

    function push(item) {
        if (queue.length >= QUEUE_MAX) queue.shift();
        queue.push(item);
    }

    function isRc28(d) {
        return d.vendorId === VENDOR && d.productId === PRODUCT;
    }

    function name(d) {
        return d.productName || "Icom RC-28";
    }

    function fail(text) {
        lastError = text;
        push("error:" + text);
    }

    function onReport(e) {
        push(new Uint8Array(e.data.buffer, e.data.byteOffset, e.data.byteLength).slice());
    }

    async function open(d) {
        if (device || opening) return;
        opening = (async function () {
            try {
                if (!d.opened) await d.open();
                if (!enabled) {
                    // Switched off while the open was in flight.
                    await d.close();
                    return;
                }
                d.addEventListener("inputreport", onReport);
                device = d;
                lastError = "";
                push("connected:" + name(d));
            } catch (e) {
                fail("cannot open the RC-28: " + (e && e.message ? e.message : e));
            }
        })();
        try {
            await opening;
        } finally {
            opening = null;
        }
    }

    async function close() {
        const d = device;
        if (!d) return;
        device = null;
        d.removeEventListener("inputreport", onReport);
        try {
            await d.close();
        } catch (e) {
            // Already gone.
        }
        push("disconnected");
    }

    async function reconnect() {
        if (!SUPPORTED || !enabled || device) return;
        try {
            const granted = (await navigator.hid.getDevices()).filter(isRc28);
            if (granted.length > 0) await open(granted[0]);
        } catch (e) {
            // No devices listed is the same as none plugged in.
        }
    }

    if (SUPPORTED) {
        navigator.hid.addEventListener("connect", function (e) {
            if (isRc28(e.device)) reconnect();
        });
        navigator.hid.addEventListener("disconnect", function (e) {
            if (e.device === device) {
                device = null;
                push("disconnected");
            }
        });
    }

    window.sdroxideRc28 = {
        supported: function () {
            return SUPPORTED;
        },
        setEnabled: function (on) {
            enabled = !!on;
            if (enabled) reconnect();
            else close();
        },
        request: function () {
            if (!SUPPORTED) return;
            navigator.hid
                .requestDevice({ filters: FILTERS })
                .then(function (picked) {
                    const d = picked.find(isRc28);
                    if (d && enabled) return open(d);
                })
                .catch(function (e) {
                    fail(e && e.message ? e.message : String(e));
                });
        },
        connectedName: function () {
            return device ? name(device) : "";
        },
        lastError: function () {
            return lastError;
        },
        drain: function () {
            const out = queue;
            queue = [];
            return out;
        },
        write: function (bytes) {
            const d = device;
            if (!d) return;
            const copy = new Uint8Array(bytes);
            writing = writing
                .then(function () {
                    return d.sendReport(0, copy);
                })
                .catch(function () {
                    // An unplug surfaces as the disconnect event above.
                });
        },
    };
})();
