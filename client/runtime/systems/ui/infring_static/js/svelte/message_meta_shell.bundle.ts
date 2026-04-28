/* generated: dashboard svelte island bundle (message_meta_shell) */
(() => {
  var __defProp = Object.defineProperty;
  var __defNormalProp = (obj, key, value) => key in obj ? __defProp(obj, key, { enumerable: true, configurable: true, writable: true, value }) : obj[key] = value;
  var __publicField = (obj, key, value) => __defNormalProp(obj, typeof key !== "symbol" ? key + "" : key, value);

  // node_modules/svelte/src/runtime/internal/utils.js
  function noop() {
  }
  function run(fn) {
    return fn();
  }
  function blank_object() {
    return /* @__PURE__ */ Object.create(null);
  }
  function run_all(fns) {
    fns.forEach(run);
  }
  function is_function(thing) {
    return typeof thing === "function";
  }
  function safe_not_equal(a, b) {
    return a != a ? b == b : a !== b || a && typeof a === "object" || typeof a === "function";
  }
  var src_url_equal_anchor;
  function src_url_equal(element_src, url) {
    if (element_src === url) return true;
    if (!src_url_equal_anchor) {
      src_url_equal_anchor = document.createElement("a");
    }
    src_url_equal_anchor.href = url;
    return element_src === src_url_equal_anchor.href;
  }
  function is_empty(obj) {
    return Object.keys(obj).length === 0;
  }

  // node_modules/svelte/src/runtime/internal/globals.js
  var globals = typeof window !== "undefined" ? window : typeof globalThis !== "undefined" ? globalThis : (
    // @ts-ignore Node typings have this
    global
  );

  // node_modules/svelte/src/runtime/internal/ResizeObserverSingleton.js
  var ResizeObserverSingleton = class _ResizeObserverSingleton {
    /** @param {ResizeObserverOptions} options */
    constructor(options) {
      /**
       * @private
       * @readonly
       * @type {WeakMap<Element, import('./private.js').Listener>}
       */
      __publicField(this, "_listeners", "WeakMap" in globals ? /* @__PURE__ */ new WeakMap() : void 0);
      /**
       * @private
       * @type {ResizeObserver}
       */
      __publicField(this, "_observer");
      /** @type {ResizeObserverOptions} */
      __publicField(this, "options");
      this.options = options;
    }
    /**
     * @param {Element} element
     * @param {import('./private.js').Listener} listener
     * @returns {() => void}
     */
    observe(element2, listener) {
      this._listeners.set(element2, listener);
      this._getObserver().observe(element2, this.options);
      return () => {
        this._listeners.delete(element2);
        this._observer.unobserve(element2);
      };
    }
    /**
     * @private
     */
    _getObserver() {
      return this._observer ?? (this._observer = new ResizeObserver((entries) => {
        for (const entry of entries) {
          _ResizeObserverSingleton.entries.set(entry.target, entry);
          this._listeners.get(entry.target)?.(entry);
        }
      }));
    }
  };
  ResizeObserverSingleton.entries = "WeakMap" in globals ? /* @__PURE__ */ new WeakMap() : void 0;

  // node_modules/svelte/src/runtime/internal/dom.js
  var is_hydrating = false;
  function start_hydrating() {
    is_hydrating = true;
  }
  function end_hydrating() {
    is_hydrating = false;
  }
  function append(target, node) {
    target.appendChild(node);
  }
  function insert(target, node, anchor) {
    target.insertBefore(node, anchor || null);
  }
  function detach(node) {
    if (node.parentNode) {
      node.parentNode.removeChild(node);
    }
  }
  function element(name) {
    return document.createElement(name);
  }
  function svg_element(name) {
    return document.createElementNS("http://www.w3.org/2000/svg", name);
  }
  function text(data) {
    return document.createTextNode(data);
  }
  function space() {
    return text(" ");
  }
  function empty() {
    return text("");
  }
  function listen(node, event, handler, options) {
    node.addEventListener(event, handler, options);
    return () => node.removeEventListener(event, handler, options);
  }
  function stop_propagation(fn) {
    return function(event) {
      event.stopPropagation();
      return fn.call(this, event);
    };
  }
  function attr(node, attribute, value) {
    if (value == null) node.removeAttribute(attribute);
    else if (node.getAttribute(attribute) !== value) node.setAttribute(attribute, value);
  }
  function children(element2) {
    return Array.from(element2.childNodes);
  }
  function set_data(text2, data) {
    data = "" + data;
    if (text2.data === data) return;
    text2.data = /** @type {string} */
    data;
  }
  function toggle_class(element2, name, toggle) {
    element2.classList.toggle(name, !!toggle);
  }
  function custom_event(type, detail, { bubbles = false, cancelable = false } = {}) {
    return new CustomEvent(type, { detail, bubbles, cancelable });
  }
  function get_custom_elements_slots(element2) {
    const result = {};
    element2.childNodes.forEach(
      /** @param {Element} node */
      (node) => {
        result[node.slot || "default"] = true;
      }
    );
    return result;
  }

  // node_modules/svelte/src/runtime/internal/lifecycle.js
  var current_component;
  function set_current_component(component) {
    current_component = component;
  }
  function get_current_component() {
    if (!current_component) throw new Error("Function called outside component initialization");
    return current_component;
  }
  function createEventDispatcher() {
    const component = get_current_component();
    return (type, detail, { cancelable = false } = {}) => {
      const callbacks = component.$$.callbacks[type];
      if (callbacks) {
        const event = custom_event(
          /** @type {string} */
          type,
          detail,
          { cancelable }
        );
        callbacks.slice().forEach((fn) => {
          fn.call(component, event);
        });
        return !event.defaultPrevented;
      }
      return true;
    };
  }

  // node_modules/svelte/src/runtime/internal/scheduler.js
  var dirty_components = [];
  var binding_callbacks = [];
  var render_callbacks = [];
  var flush_callbacks = [];
  var resolved_promise = /* @__PURE__ */ Promise.resolve();
  var update_scheduled = false;
  function schedule_update() {
    if (!update_scheduled) {
      update_scheduled = true;
      resolved_promise.then(flush);
    }
  }
  function add_render_callback(fn) {
    render_callbacks.push(fn);
  }
  var seen_callbacks = /* @__PURE__ */ new Set();
  var flushidx = 0;
  function flush() {
    if (flushidx !== 0) {
      return;
    }
    const saved_component = current_component;
    do {
      try {
        while (flushidx < dirty_components.length) {
          const component = dirty_components[flushidx];
          flushidx++;
          set_current_component(component);
          update(component.$$);
        }
      } catch (e) {
        dirty_components.length = 0;
        flushidx = 0;
        throw e;
      }
      set_current_component(null);
      dirty_components.length = 0;
      flushidx = 0;
      while (binding_callbacks.length) binding_callbacks.pop()();
      for (let i = 0; i < render_callbacks.length; i += 1) {
        const callback = render_callbacks[i];
        if (!seen_callbacks.has(callback)) {
          seen_callbacks.add(callback);
          callback();
        }
      }
      render_callbacks.length = 0;
    } while (dirty_components.length);
    while (flush_callbacks.length) {
      flush_callbacks.pop()();
    }
    update_scheduled = false;
    seen_callbacks.clear();
    set_current_component(saved_component);
  }
  function update($$) {
    if ($$.fragment !== null) {
      $$.update();
      run_all($$.before_update);
      const dirty = $$.dirty;
      $$.dirty = [-1];
      $$.fragment && $$.fragment.p($$.ctx, dirty);
      $$.after_update.forEach(add_render_callback);
    }
  }
  function flush_render_callbacks(fns) {
    const filtered = [];
    const targets = [];
    render_callbacks.forEach((c) => fns.indexOf(c) === -1 ? filtered.push(c) : targets.push(c));
    targets.forEach((c) => c());
    render_callbacks = filtered;
  }

  // node_modules/svelte/src/runtime/internal/transitions.js
  var outroing = /* @__PURE__ */ new Set();
  function transition_in(block, local) {
    if (block && block.i) {
      outroing.delete(block);
      block.i(local);
    }
  }

  // node_modules/svelte/src/shared/boolean_attributes.js
  var _boolean_attributes = (
    /** @type {const} */
    [
      "allowfullscreen",
      "allowpaymentrequest",
      "async",
      "autofocus",
      "autoplay",
      "checked",
      "controls",
      "default",
      "defer",
      "disabled",
      "formnovalidate",
      "hidden",
      "inert",
      "ismap",
      "loop",
      "multiple",
      "muted",
      "nomodule",
      "novalidate",
      "open",
      "playsinline",
      "readonly",
      "required",
      "reversed",
      "selected"
    ]
  );
  var boolean_attributes = /* @__PURE__ */ new Set([..._boolean_attributes]);

  // node_modules/svelte/src/runtime/internal/Component.js
  function mount_component(component, target, anchor) {
    const { fragment, after_update } = component.$$;
    fragment && fragment.m(target, anchor);
    add_render_callback(() => {
      const new_on_destroy = component.$$.on_mount.map(run).filter(is_function);
      if (component.$$.on_destroy) {
        component.$$.on_destroy.push(...new_on_destroy);
      } else {
        run_all(new_on_destroy);
      }
      component.$$.on_mount = [];
    });
    after_update.forEach(add_render_callback);
  }
  function destroy_component(component, detaching) {
    const $$ = component.$$;
    if ($$.fragment !== null) {
      flush_render_callbacks($$.after_update);
      run_all($$.on_destroy);
      $$.fragment && $$.fragment.d(detaching);
      $$.on_destroy = $$.fragment = null;
      $$.ctx = [];
    }
  }
  function make_dirty(component, i) {
    if (component.$$.dirty[0] === -1) {
      dirty_components.push(component);
      schedule_update();
      component.$$.dirty.fill(0);
    }
    component.$$.dirty[i / 31 | 0] |= 1 << i % 31;
  }
  function init(component, options, instance2, create_fragment2, not_equal, props, append_styles = null, dirty = [-1]) {
    const parent_component = current_component;
    set_current_component(component);
    const $$ = component.$$ = {
      fragment: null,
      ctx: [],
      // state
      props,
      update: noop,
      not_equal,
      bound: blank_object(),
      // lifecycle
      on_mount: [],
      on_destroy: [],
      on_disconnect: [],
      before_update: [],
      after_update: [],
      context: new Map(options.context || (parent_component ? parent_component.$$.context : [])),
      // everything else
      callbacks: blank_object(),
      dirty,
      skip_bound: false,
      root: options.target || parent_component.$$.root
    };
    append_styles && append_styles($$.root);
    let ready = false;
    $$.ctx = instance2 ? instance2(component, options.props || {}, (i, ret, ...rest) => {
      const value = rest.length ? rest[0] : ret;
      if ($$.ctx && not_equal($$.ctx[i], $$.ctx[i] = value)) {
        if (!$$.skip_bound && $$.bound[i]) $$.bound[i](value);
        if (ready) make_dirty(component, i);
      }
      return ret;
    }) : [];
    $$.update();
    ready = true;
    run_all($$.before_update);
    $$.fragment = create_fragment2 ? create_fragment2($$.ctx) : false;
    if (options.target) {
      if (options.hydrate) {
        start_hydrating();
        const nodes = children(options.target);
        $$.fragment && $$.fragment.l(nodes);
        nodes.forEach(detach);
      } else {
        $$.fragment && $$.fragment.c();
      }
      if (options.intro) transition_in(component.$$.fragment);
      mount_component(component, options.target, options.anchor);
      end_hydrating();
      flush();
    }
    set_current_component(parent_component);
  }
  var SvelteElement;
  if (typeof HTMLElement === "function") {
    SvelteElement = class extends HTMLElement {
      constructor($$componentCtor, $$slots, use_shadow_dom) {
        super();
        /** The Svelte component constructor */
        __publicField(this, "$$ctor");
        /** Slots */
        __publicField(this, "$$s");
        /** The Svelte component instance */
        __publicField(this, "$$c");
        /** Whether or not the custom element is connected */
        __publicField(this, "$$cn", false);
        /** Component props data */
        __publicField(this, "$$d", {});
        /** `true` if currently in the process of reflecting component props back to attributes */
        __publicField(this, "$$r", false);
        /** @type {Record<string, CustomElementPropDefinition>} Props definition (name, reflected, type etc) */
        __publicField(this, "$$p_d", {});
        /** @type {Record<string, Function[]>} Event listeners */
        __publicField(this, "$$l", {});
        /** @type {Map<Function, Function>} Event listener unsubscribe functions */
        __publicField(this, "$$l_u", /* @__PURE__ */ new Map());
        this.$$ctor = $$componentCtor;
        this.$$s = $$slots;
        if (use_shadow_dom) {
          this.attachShadow({ mode: "open" });
        }
      }
      addEventListener(type, listener, options) {
        this.$$l[type] = this.$$l[type] || [];
        this.$$l[type].push(listener);
        if (this.$$c) {
          const unsub = this.$$c.$on(type, listener);
          this.$$l_u.set(listener, unsub);
        }
        super.addEventListener(type, listener, options);
      }
      removeEventListener(type, listener, options) {
        super.removeEventListener(type, listener, options);
        if (this.$$c) {
          const unsub = this.$$l_u.get(listener);
          if (unsub) {
            unsub();
            this.$$l_u.delete(listener);
          }
        }
        if (this.$$l[type]) {
          const idx = this.$$l[type].indexOf(listener);
          if (idx >= 0) {
            this.$$l[type].splice(idx, 1);
          }
        }
      }
      async connectedCallback() {
        this.$$cn = true;
        if (!this.$$c) {
          let create_slot = function(name) {
            return () => {
              let node;
              const obj = {
                c: function create() {
                  node = element("slot");
                  if (name !== "default") {
                    attr(node, "name", name);
                  }
                },
                /**
                 * @param {HTMLElement} target
                 * @param {HTMLElement} [anchor]
                 */
                m: function mount(target, anchor) {
                  insert(target, node, anchor);
                },
                d: function destroy(detaching) {
                  if (detaching) {
                    detach(node);
                  }
                }
              };
              return obj;
            };
          };
          await Promise.resolve();
          if (!this.$$cn || this.$$c) {
            return;
          }
          const $$slots = {};
          const existing_slots = get_custom_elements_slots(this);
          for (const name of this.$$s) {
            if (name in existing_slots) {
              $$slots[name] = [create_slot(name)];
            }
          }
          for (const attribute of this.attributes) {
            const name = this.$$g_p(attribute.name);
            if (!(name in this.$$d)) {
              this.$$d[name] = get_custom_element_value(name, attribute.value, this.$$p_d, "toProp");
            }
          }
          for (const key in this.$$p_d) {
            if (!(key in this.$$d) && this[key] !== void 0) {
              this.$$d[key] = this[key];
              delete this[key];
            }
          }
          this.$$c = new this.$$ctor({
            target: this.shadowRoot || this,
            props: {
              ...this.$$d,
              $$slots,
              $$scope: {
                ctx: []
              }
            }
          });
          const reflect_attributes = () => {
            this.$$r = true;
            for (const key in this.$$p_d) {
              this.$$d[key] = this.$$c.$$.ctx[this.$$c.$$.props[key]];
              if (this.$$p_d[key].reflect) {
                const attribute_value = get_custom_element_value(
                  key,
                  this.$$d[key],
                  this.$$p_d,
                  "toAttribute"
                );
                if (attribute_value == null) {
                  this.removeAttribute(this.$$p_d[key].attribute || key);
                } else {
                  this.setAttribute(this.$$p_d[key].attribute || key, attribute_value);
                }
              }
            }
            this.$$r = false;
          };
          this.$$c.$$.after_update.push(reflect_attributes);
          reflect_attributes();
          for (const type in this.$$l) {
            for (const listener of this.$$l[type]) {
              const unsub = this.$$c.$on(type, listener);
              this.$$l_u.set(listener, unsub);
            }
          }
          this.$$l = {};
        }
      }
      // We don't need this when working within Svelte code, but for compatibility of people using this outside of Svelte
      // and setting attributes through setAttribute etc, this is helpful
      attributeChangedCallback(attr2, _oldValue, newValue) {
        if (this.$$r) return;
        attr2 = this.$$g_p(attr2);
        this.$$d[attr2] = get_custom_element_value(attr2, newValue, this.$$p_d, "toProp");
        this.$$c?.$set({ [attr2]: this.$$d[attr2] });
      }
      disconnectedCallback() {
        this.$$cn = false;
        Promise.resolve().then(() => {
          if (!this.$$cn && this.$$c) {
            this.$$c.$destroy();
            this.$$c = void 0;
          }
        });
      }
      $$g_p(attribute_name) {
        return Object.keys(this.$$p_d).find(
          (key) => this.$$p_d[key].attribute === attribute_name || !this.$$p_d[key].attribute && key.toLowerCase() === attribute_name
        ) || attribute_name;
      }
    };
  }
  function get_custom_element_value(prop, value, props_definition, transform) {
    const type = props_definition[prop]?.type;
    value = type === "Boolean" && typeof value !== "boolean" ? value != null : value;
    if (!transform || !props_definition[prop]) {
      return value;
    } else if (transform === "toAttribute") {
      switch (type) {
        case "Object":
        case "Array":
          return value == null ? null : JSON.stringify(value);
        case "Boolean":
          return value ? "" : null;
        case "Number":
          return value == null ? null : value;
        default:
          return value;
      }
    } else {
      switch (type) {
        case "Object":
        case "Array":
          return value && JSON.parse(value);
        case "Boolean":
          return value;
        case "Number":
          return value != null ? +value : value;
        default:
          return value;
      }
    }
  }
  function create_custom_element(Component, props_definition, slots, accessors, use_shadow_dom, extend) {
    let Class = class extends SvelteElement {
      constructor() {
        super(Component, slots, use_shadow_dom);
        this.$$p_d = props_definition;
      }
      static get observedAttributes() {
        return Object.keys(props_definition).map(
          (key) => (props_definition[key].attribute || key).toLowerCase()
        );
      }
    };
    Object.keys(props_definition).forEach((prop) => {
      Object.defineProperty(Class.prototype, prop, {
        get() {
          return this.$$c && prop in this.$$c ? this.$$c[prop] : this.$$d[prop];
        },
        set(value) {
          value = get_custom_element_value(prop, value, props_definition);
          this.$$d[prop] = value;
          this.$$c?.$set({ [prop]: value });
        }
      });
    });
    accessors.forEach((accessor) => {
      Object.defineProperty(Class.prototype, accessor, {
        get() {
          return this.$$c?.[accessor];
        }
      });
    });
    if (extend) {
      Class = extend(Class);
    }
    Component.element = /** @type {any} */
    Class;
    return Class;
  }
  var SvelteComponent = class {
    constructor() {
      /**
       * ### PRIVATE API
       *
       * Do not use, may change at any time
       *
       * @type {any}
       */
      __publicField(this, "$$");
      /**
       * ### PRIVATE API
       *
       * Do not use, may change at any time
       *
       * @type {any}
       */
      __publicField(this, "$$set");
    }
    /** @returns {void} */
    $destroy() {
      destroy_component(this, 1);
      this.$destroy = noop;
    }
    /**
     * @template {Extract<keyof Events, string>} K
     * @param {K} type
     * @param {((e: Events[K]) => void) | null | undefined} callback
     * @returns {() => void}
     */
    $on(type, callback) {
      if (!is_function(callback)) {
        return noop;
      }
      const callbacks = this.$$.callbacks[type] || (this.$$.callbacks[type] = []);
      callbacks.push(callback);
      return () => {
        const index = callbacks.indexOf(callback);
        if (index !== -1) callbacks.splice(index, 1);
      };
    }
    /**
     * @param {Partial<Props>} props
     * @returns {void}
     */
    $set(props) {
      if (this.$$set && !is_empty(props)) {
        this.$$.skip_bound = true;
        this.$$set(props);
        this.$$.skip_bound = false;
      }
    }
  };

  // node_modules/svelte/src/shared/version.js
  var PUBLIC_VERSION = "4";

  // node_modules/svelte/src/runtime/internal/disclose-version/index.js
  if (typeof window !== "undefined")
    (window.__svelte || (window.__svelte = { v: /* @__PURE__ */ new Set() })).v.add(PUBLIC_VERSION);

  // message_meta_shell.svelte.js
  function create_if_block(ctx) {
    let div;
    let button;
    let button_title_value;
    let button_aria_label_value;
    let t0;
    let t1;
    let t2;
    let t3;
    let t4;
    let t5;
    let t6;
    let t7;
    let mounted;
    let dispose;
    function select_block_type(ctx2, dirty) {
      if (
        /*model*/
        ctx2[0].copied
      ) return create_if_block_10;
      return create_else_block_1;
    }
    let current_block_type = select_block_type(ctx, -1);
    let if_block0 = current_block_type(ctx);
    let if_block1 = (
      /*model*/
      ctx[0].canReportIssue && create_if_block_9(ctx)
    );
    let if_block2 = (
      /*model*/
      ctx[0].hasTools && create_if_block_7(ctx)
    );
    let if_block3 = (
      /*model*/
      ctx[0].canRetry && create_if_block_6(ctx)
    );
    let if_block4 = (
      /*model*/
      ctx[0].canReply && create_if_block_5(ctx)
    );
    let if_block5 = (
      /*model*/
      ctx[0].canFork && create_if_block_4(ctx)
    );
    let if_block6 = (
      /*model*/
      ctx[0].visible && /*model*/
      ctx[0].timestamp && create_if_block_3(ctx)
    );
    let if_block7 = (
      /*model*/
      ctx[0].visible && /*model*/
      ctx[0].responseTime && create_if_block_2(ctx)
    );
    let if_block8 = (
      /*model*/
      ctx[0].visible && /*model*/
      ctx[0].burnLabel && create_if_block_1(ctx)
    );
    return {
      c() {
        div = element("div");
        button = element("button");
        if_block0.c();
        t0 = space();
        if (if_block1) if_block1.c();
        t1 = space();
        if (if_block2) if_block2.c();
        t2 = space();
        if (if_block3) if_block3.c();
        t3 = space();
        if (if_block4) if_block4.c();
        t4 = space();
        if (if_block5) if_block5.c();
        t5 = space();
        if (if_block6) if_block6.c();
        t6 = space();
        if (if_block7) if_block7.c();
        t7 = space();
        if (if_block8) if_block8.c();
        attr(button, "type", "button");
        attr(button, "class", "message-stat-btn");
        attr(button, "title", button_title_value = /*model*/
        ctx[0].copied ? "Copied" : "Copy message");
        attr(button, "aria-label", button_aria_label_value = /*model*/
        ctx[0].copied ? "Copied" : "Copy message");
        toggle_class(
          button,
          "copied",
          /*model*/
          ctx[0].copied
        );
        attr(div, "class", "message-stats-row");
      },
      m(target, anchor) {
        insert(target, div, anchor);
        append(div, button);
        if_block0.m(button, null);
        append(div, t0);
        if (if_block1) if_block1.m(div, null);
        append(div, t1);
        if (if_block2) if_block2.m(div, null);
        append(div, t2);
        if (if_block3) if_block3.m(div, null);
        append(div, t3);
        if (if_block4) if_block4.m(div, null);
        append(div, t4);
        if (if_block5) if_block5.m(div, null);
        append(div, t5);
        if (if_block6) if_block6.m(div, null);
        append(div, t6);
        if (if_block7) if_block7.m(div, null);
        append(div, t7);
        if (if_block8) if_block8.m(div, null);
        if (!mounted) {
          dispose = listen(
            button,
            "click",
            /*click_handler*/
            ctx[3]
          );
          mounted = true;
        }
      },
      p(ctx2, dirty) {
        if (current_block_type !== (current_block_type = select_block_type(ctx2, dirty))) {
          if_block0.d(1);
          if_block0 = current_block_type(ctx2);
          if (if_block0) {
            if_block0.c();
            if_block0.m(button, null);
          }
        }
        if (dirty & /*model*/
        1 && button_title_value !== (button_title_value = /*model*/
        ctx2[0].copied ? "Copied" : "Copy message")) {
          attr(button, "title", button_title_value);
        }
        if (dirty & /*model*/
        1 && button_aria_label_value !== (button_aria_label_value = /*model*/
        ctx2[0].copied ? "Copied" : "Copy message")) {
          attr(button, "aria-label", button_aria_label_value);
        }
        if (dirty & /*model*/
        1) {
          toggle_class(
            button,
            "copied",
            /*model*/
            ctx2[0].copied
          );
        }
        if (
          /*model*/
          ctx2[0].canReportIssue
        ) {
          if (if_block1) {
            if_block1.p(ctx2, dirty);
          } else {
            if_block1 = create_if_block_9(ctx2);
            if_block1.c();
            if_block1.m(div, t1);
          }
        } else if (if_block1) {
          if_block1.d(1);
          if_block1 = null;
        }
        if (
          /*model*/
          ctx2[0].hasTools
        ) {
          if (if_block2) {
            if_block2.p(ctx2, dirty);
          } else {
            if_block2 = create_if_block_7(ctx2);
            if_block2.c();
            if_block2.m(div, t2);
          }
        } else if (if_block2) {
          if_block2.d(1);
          if_block2 = null;
        }
        if (
          /*model*/
          ctx2[0].canRetry
        ) {
          if (if_block3) {
            if_block3.p(ctx2, dirty);
          } else {
            if_block3 = create_if_block_6(ctx2);
            if_block3.c();
            if_block3.m(div, t3);
          }
        } else if (if_block3) {
          if_block3.d(1);
          if_block3 = null;
        }
        if (
          /*model*/
          ctx2[0].canReply
        ) {
          if (if_block4) {
            if_block4.p(ctx2, dirty);
          } else {
            if_block4 = create_if_block_5(ctx2);
            if_block4.c();
            if_block4.m(div, t4);
          }
        } else if (if_block4) {
          if_block4.d(1);
          if_block4 = null;
        }
        if (
          /*model*/
          ctx2[0].canFork
        ) {
          if (if_block5) {
            if_block5.p(ctx2, dirty);
          } else {
            if_block5 = create_if_block_4(ctx2);
            if_block5.c();
            if_block5.m(div, t5);
          }
        } else if (if_block5) {
          if_block5.d(1);
          if_block5 = null;
        }
        if (
          /*model*/
          ctx2[0].visible && /*model*/
          ctx2[0].timestamp
        ) {
          if (if_block6) {
            if_block6.p(ctx2, dirty);
          } else {
            if_block6 = create_if_block_3(ctx2);
            if_block6.c();
            if_block6.m(div, t6);
          }
        } else if (if_block6) {
          if_block6.d(1);
          if_block6 = null;
        }
        if (
          /*model*/
          ctx2[0].visible && /*model*/
          ctx2[0].responseTime
        ) {
          if (if_block7) {
            if_block7.p(ctx2, dirty);
          } else {
            if_block7 = create_if_block_2(ctx2);
            if_block7.c();
            if_block7.m(div, t7);
          }
        } else if (if_block7) {
          if_block7.d(1);
          if_block7 = null;
        }
        if (
          /*model*/
          ctx2[0].visible && /*model*/
          ctx2[0].burnLabel
        ) {
          if (if_block8) {
            if_block8.p(ctx2, dirty);
          } else {
            if_block8 = create_if_block_1(ctx2);
            if_block8.c();
            if_block8.m(div, null);
          }
        } else if (if_block8) {
          if_block8.d(1);
          if_block8 = null;
        }
      },
      d(detaching) {
        if (detaching) {
          detach(div);
        }
        if_block0.d();
        if (if_block1) if_block1.d();
        if (if_block2) if_block2.d();
        if (if_block3) if_block3.d();
        if (if_block4) if_block4.d();
        if (if_block5) if_block5.d();
        if (if_block6) if_block6.d();
        if (if_block7) if_block7.d();
        if (if_block8) if_block8.d();
        mounted = false;
        dispose();
      }
    };
  }
  function create_else_block_1(ctx) {
    let svg;
    let rect;
    let path;
    return {
      c() {
        svg = svg_element("svg");
        rect = svg_element("rect");
        path = svg_element("path");
        attr(rect, "x", "9");
        attr(rect, "y", "9");
        attr(rect, "width", "13");
        attr(rect, "height", "13");
        attr(rect, "rx", "2");
        attr(path, "d", "M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1");
        attr(svg, "width", "13");
        attr(svg, "height", "13");
        attr(svg, "viewBox", "0 0 24 24");
        attr(svg, "fill", "none");
        attr(svg, "stroke", "currentColor");
        attr(svg, "stroke-width", "2");
        attr(svg, "stroke-linecap", "round");
        attr(svg, "stroke-linejoin", "round");
      },
      m(target, anchor) {
        insert(target, svg, anchor);
        append(svg, rect);
        append(svg, path);
      },
      d(detaching) {
        if (detaching) {
          detach(svg);
        }
      }
    };
  }
  function create_if_block_10(ctx) {
    let svg;
    let path;
    return {
      c() {
        svg = svg_element("svg");
        path = svg_element("path");
        attr(path, "d", "M20 6L9 17l-5-5");
        attr(svg, "width", "13");
        attr(svg, "height", "13");
        attr(svg, "viewBox", "0 0 24 24");
        attr(svg, "fill", "none");
        attr(svg, "stroke", "currentColor");
        attr(svg, "stroke-width", "2.4");
        attr(svg, "stroke-linecap", "round");
        attr(svg, "stroke-linejoin", "round");
      },
      m(target, anchor) {
        insert(target, svg, anchor);
        append(svg, path);
      },
      d(detaching) {
        if (detaching) {
          detach(svg);
        }
      }
    };
  }
  function create_if_block_9(ctx) {
    let button;
    let mounted;
    let dispose;
    return {
      c() {
        button = element("button");
        button.innerHTML = `<svg class="message-stat-icon message-stat-icon-hazard" width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.1" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M10.3 3.9 1.8 18.4A2 2 0 0 0 3.5 21h17a2 2 0 0 0 1.7-2.6L13.7 3.9a2 2 0 0 0-3.4 0Z"></path><path d="M12 9v5"></path><path d="M12 17h.01"></path></svg>`;
        attr(button, "type", "button");
        attr(button, "class", "message-stat-btn message-action-report-issue");
        attr(button, "title", "Send this chat context to eval review");
        attr(button, "aria-label", "Send this chat context to eval review");
      },
      m(target, anchor) {
        insert(target, button, anchor);
        if (!mounted) {
          dispose = listen(button, "click", stop_propagation(
            /*click_handler_1*/
            ctx[4]
          ));
          mounted = true;
        }
      },
      p: noop,
      d(detaching) {
        if (detaching) {
          detach(button);
        }
        mounted = false;
        dispose();
      }
    };
  }
  function create_if_block_7(ctx) {
    let button;
    let button_title_value;
    let button_aria_label_value;
    let mounted;
    let dispose;
    function select_block_type_1(ctx2, dirty) {
      if (
        /*model*/
        ctx2[0].toolsCollapsed
      ) return create_if_block_8;
      return create_else_block;
    }
    let current_block_type = select_block_type_1(ctx, -1);
    let if_block = current_block_type(ctx);
    return {
      c() {
        button = element("button");
        if_block.c();
        attr(button, "type", "button");
        attr(button, "class", "message-stat-btn");
        attr(button, "title", button_title_value = /*model*/
        ctx[0].toolsCollapsed ? "Expand processes" : "Collapse processes");
        attr(button, "aria-label", button_aria_label_value = /*model*/
        ctx[0].toolsCollapsed ? "Expand processes" : "Collapse processes");
      },
      m(target, anchor) {
        insert(target, button, anchor);
        if_block.m(button, null);
        if (!mounted) {
          dispose = listen(
            button,
            "click",
            /*click_handler_2*/
            ctx[5]
          );
          mounted = true;
        }
      },
      p(ctx2, dirty) {
        if (current_block_type !== (current_block_type = select_block_type_1(ctx2, dirty))) {
          if_block.d(1);
          if_block = current_block_type(ctx2);
          if (if_block) {
            if_block.c();
            if_block.m(button, null);
          }
        }
        if (dirty & /*model*/
        1 && button_title_value !== (button_title_value = /*model*/
        ctx2[0].toolsCollapsed ? "Expand processes" : "Collapse processes")) {
          attr(button, "title", button_title_value);
        }
        if (dirty & /*model*/
        1 && button_aria_label_value !== (button_aria_label_value = /*model*/
        ctx2[0].toolsCollapsed ? "Expand processes" : "Collapse processes")) {
          attr(button, "aria-label", button_aria_label_value);
        }
      },
      d(detaching) {
        if (detaching) {
          detach(button);
        }
        if_block.d();
        mounted = false;
        dispose();
      }
    };
  }
  function create_else_block(ctx) {
    let svg;
    let path;
    return {
      c() {
        svg = svg_element("svg");
        path = svg_element("path");
        attr(path, "d", "m6 9 6 6 6-6");
        attr(svg, "width", "12");
        attr(svg, "height", "12");
        attr(svg, "viewBox", "0 0 24 24");
        attr(svg, "fill", "none");
        attr(svg, "stroke", "currentColor");
        attr(svg, "stroke-width", "2.4");
        attr(svg, "stroke-linecap", "round");
        attr(svg, "stroke-linejoin", "round");
      },
      m(target, anchor) {
        insert(target, svg, anchor);
        append(svg, path);
      },
      d(detaching) {
        if (detaching) {
          detach(svg);
        }
      }
    };
  }
  function create_if_block_8(ctx) {
    let svg;
    let path;
    return {
      c() {
        svg = svg_element("svg");
        path = svg_element("path");
        attr(path, "d", "m9 6 6 6-6 6");
        attr(svg, "width", "12");
        attr(svg, "height", "12");
        attr(svg, "viewBox", "0 0 24 24");
        attr(svg, "fill", "none");
        attr(svg, "stroke", "currentColor");
        attr(svg, "stroke-width", "2.4");
        attr(svg, "stroke-linecap", "round");
        attr(svg, "stroke-linejoin", "round");
      },
      m(target, anchor) {
        insert(target, svg, anchor);
        append(svg, path);
      },
      d(detaching) {
        if (detaching) {
          detach(svg);
        }
      }
    };
  }
  function create_if_block_6(ctx) {
    let button;
    let mounted;
    let dispose;
    return {
      c() {
        button = element("button");
        button.innerHTML = `<svg class="message-stat-icon message-stat-icon-refresh" width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.1" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M21 12a9 9 0 1 1-9-9c2.52 0 4.93 1 6.74 2.74L21 8"></path><path d="M21 3v5h-5"></path></svg>`;
        attr(button, "type", "button");
        attr(button, "class", "message-stat-btn");
        attr(button, "title", "Retry from this turn");
        attr(button, "aria-label", "Retry from this turn");
      },
      m(target, anchor) {
        insert(target, button, anchor);
        if (!mounted) {
          dispose = listen(
            button,
            "click",
            /*click_handler_3*/
            ctx[6]
          );
          mounted = true;
        }
      },
      p: noop,
      d(detaching) {
        if (detaching) {
          detach(button);
        }
        mounted = false;
        dispose();
      }
    };
  }
  function create_if_block_5(ctx) {
    let button;
    let mounted;
    let dispose;
    return {
      c() {
        button = element("button");
        button.innerHTML = `<svg class="message-stat-icon message-stat-icon-reply" width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.1" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="m9 14-5-5 5-5"></path><path d="M20 20v-5a6 6 0 0 0-6-6H4"></path></svg><span class="message-reply-label">Reply</span>`;
        attr(button, "type", "button");
        attr(button, "class", "message-stat-btn message-action-reply");
        attr(button, "title", "Reply to this message");
        attr(button, "aria-label", "Reply to this message");
      },
      m(target, anchor) {
        insert(target, button, anchor);
        if (!mounted) {
          dispose = listen(
            button,
            "click",
            /*click_handler_4*/
            ctx[7]
          );
          mounted = true;
        }
      },
      p: noop,
      d(detaching) {
        if (detaching) {
          detach(button);
        }
        mounted = false;
        dispose();
      }
    };
  }
  function create_if_block_4(ctx) {
    let button;
    let mounted;
    let dispose;
    return {
      c() {
        button = element("button");
        button.innerHTML = `<svg class="message-stat-icon message-stat-icon-fork" width="13" height="13" viewBox="0 0 16 16" fill="currentColor" aria-hidden="true"><path d="M5 5.372v.878c0 .414.336.75.75.75h4.5a.75.75 0 0 0 .75-.75v-.878a2.25 2.25 0 1 1 1.5 0v.878a2.25 2.25 0 0 1-2.25 2.25h-1.5v2.128a2.251 2.251 0 1 1-1.5 0V8.5h-1.5A2.25 2.25 0 0 1 3.5 6.25v-.878a2.25 2.25 0 1 1 1.5 0ZM5 3.25a.75.75 0 1 0-1.5 0 .75.75 0 0 0 1.5 0Zm6.75.75a.75.75 0 1 0 0-1.5.75.75 0 0 0 0 1.5Zm-3 8.75a.75.75 0 1 0-1.5 0 .75.75 0 0 0 1.5 0Z"></path></svg>`;
        attr(button, "type", "button");
        attr(button, "class", "message-stat-btn");
        attr(button, "title", "Fork to a new agent");
        attr(button, "aria-label", "Fork to a new agent");
      },
      m(target, anchor) {
        insert(target, button, anchor);
        if (!mounted) {
          dispose = listen(
            button,
            "click",
            /*click_handler_5*/
            ctx[8]
          );
          mounted = true;
        }
      },
      p: noop,
      d(detaching) {
        if (detaching) {
          detach(button);
        }
        mounted = false;
        dispose();
      }
    };
  }
  function create_if_block_3(ctx) {
    let span;
    let t_value = (
      /*model*/
      ctx[0].timestamp + ""
    );
    let t;
    return {
      c() {
        span = element("span");
        t = text(t_value);
        attr(span, "class", "message-stat-time");
      },
      m(target, anchor) {
        insert(target, span, anchor);
        append(span, t);
      },
      p(ctx2, dirty) {
        if (dirty & /*model*/
        1 && t_value !== (t_value = /*model*/
        ctx2[0].timestamp + "")) set_data(t, t_value);
      },
      d(detaching) {
        if (detaching) {
          detach(span);
        }
      }
    };
  }
  function create_if_block_2(ctx) {
    let span;
    let t_value = (
      /*model*/
      ctx[0].responseTime + ""
    );
    let t;
    return {
      c() {
        span = element("span");
        t = text(t_value);
        attr(span, "class", "message-stat-meta");
      },
      m(target, anchor) {
        insert(target, span, anchor);
        append(span, t);
      },
      p(ctx2, dirty) {
        if (dirty & /*model*/
        1 && t_value !== (t_value = /*model*/
        ctx2[0].responseTime + "")) set_data(t, t_value);
      },
      d(detaching) {
        if (detaching) {
          detach(span);
        }
      }
    };
  }
  function create_if_block_1(ctx) {
    let span1;
    let img;
    let img_src_value;
    let span0;
    let t_value = (
      /*model*/
      ctx[0].burnLabel + ""
    );
    let t;
    return {
      c() {
        span1 = element("span");
        img = element("img");
        span0 = element("span");
        t = text(t_value);
        attr(img, "class", "message-meta-icon message-stat-burn-icon");
        if (!src_url_equal(img.src, img_src_value = /*model*/
        ctx[0].burnIconSrc)) attr(img, "src", img_src_value);
        attr(img, "alt", "");
        attr(img, "aria-hidden", "true");
        attr(span1, "class", "message-stat-burn");
      },
      m(target, anchor) {
        insert(target, span1, anchor);
        append(span1, img);
        append(span1, span0);
        append(span0, t);
      },
      p(ctx2, dirty) {
        if (dirty & /*model*/
        1 && !src_url_equal(img.src, img_src_value = /*model*/
        ctx2[0].burnIconSrc)) {
          attr(img, "src", img_src_value);
        }
        if (dirty & /*model*/
        1 && t_value !== (t_value = /*model*/
        ctx2[0].burnLabel + "")) set_data(t, t_value);
      },
      d(detaching) {
        if (detaching) {
          detach(span1);
        }
      }
    };
  }
  function create_fragment(ctx) {
    let if_block_anchor;
    let if_block = (
      /*model*/
      ctx[0].shouldRender && create_if_block(ctx)
    );
    return {
      c() {
        if (if_block) if_block.c();
        if_block_anchor = empty();
      },
      m(target, anchor) {
        if (if_block) if_block.m(target, anchor);
        insert(target, if_block_anchor, anchor);
      },
      p(ctx2, [dirty]) {
        if (
          /*model*/
          ctx2[0].shouldRender
        ) {
          if (if_block) {
            if_block.p(ctx2, dirty);
          } else {
            if_block = create_if_block(ctx2);
            if_block.c();
            if_block.m(if_block_anchor.parentNode, if_block_anchor);
          }
        } else if (if_block) {
          if_block.d(1);
          if_block = null;
        }
      },
      i: noop,
      o: noop,
      d(detaching) {
        if (detaching) {
          detach(if_block_anchor);
        }
        if (if_block) if_block.d(detaching);
      }
    };
  }
  function asBoolean(value) {
    if (value === true || value === false) return value;
    const text2 = String(value == null ? "" : value).trim().toLowerCase();
    return text2 === "1" || text2 === "true" || text2 === "yes" || text2 === "on";
  }
  function asText(value) {
    return String(value == null ? "" : value).trim();
  }
  function parseState(value) {
    if (value && typeof value === "object") return value;
    const text2 = String(value == null ? "" : value).trim();
    if (!text2) return {};
    try {
      const parsed = JSON.parse(text2);
      return parsed && typeof parsed === "object" ? parsed : {};
    } catch (_) {
      return {};
    }
  }
  function normalizeState(value) {
    const source = parseState(value);
    return {
      shouldRender: asBoolean(source.shouldRender),
      visible: asBoolean(source.visible),
      copied: asBoolean(source.copied),
      hasTools: asBoolean(source.hasTools),
      toolsCollapsed: asBoolean(source.toolsCollapsed),
      canReportIssue: asBoolean(source.canReportIssue),
      canRetry: asBoolean(source.canRetry),
      canReply: asBoolean(source.canReply),
      canFork: asBoolean(source.canFork),
      timestamp: asText(source.timestamp),
      responseTime: asText(source.responseTime),
      burnLabel: asText(source.burnLabel),
      burnIconSrc: asText(source.burnIconSrc) || "/icons/vecteezy_fire-icon-simple-vector-perfect-illustration_13821331.svg"
    };
  }
  function instance($$self, $$props, $$invalidate) {
    let model;
    let { state = "" } = $$props;
    const dispatch = createEventDispatcher();
    function emit(action) {
      dispatch("message-meta-action", { action });
    }
    const click_handler = () => emit("copy");
    const click_handler_1 = () => emit("report");
    const click_handler_2 = () => emit("toggle-tools");
    const click_handler_3 = () => emit("retry");
    const click_handler_4 = () => emit("reply");
    const click_handler_5 = () => emit("fork");
    $$self.$$set = ($$props2) => {
      if ("state" in $$props2) $$invalidate(2, state = $$props2.state);
    };
    $$self.$$.update = () => {
      if ($$self.$$.dirty & /*state*/
      4) {
        $: $$invalidate(0, model = normalizeState(state));
      }
    };
    return [
      model,
      emit,
      state,
      click_handler,
      click_handler_1,
      click_handler_2,
      click_handler_3,
      click_handler_4,
      click_handler_5
    ];
  }
  var Message_meta_shell = class extends SvelteComponent {
    constructor(options) {
      super();
      init(this, options, instance, create_fragment, safe_not_equal, { state: 2 });
    }
    get state() {
      return this.$$.ctx[2];
    }
    set state(state) {
      this.$$set({ state });
      flush();
    }
  };
  customElements.define("infring-message-meta-shell", create_custom_element(Message_meta_shell, { "state": {} }, [], [], false));
  var message_meta_shell_svelte_default = Message_meta_shell;
})();
