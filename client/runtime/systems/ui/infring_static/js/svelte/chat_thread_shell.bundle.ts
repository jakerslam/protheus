/* generated: dashboard svelte island bundle (chat_thread_shell) */
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
  function destroy_each(iterations, detaching) {
    for (let i = 0; i < iterations.length; i += 1) {
      if (iterations[i]) iterations[i].d(detaching);
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
  function set_custom_element_data(node, prop, value) {
    const lower = prop.toLowerCase();
    if (lower in node) {
      node[lower] = typeof node[lower] === "boolean" && value === "" ? true : value;
    } else if (prop in node) {
      node[prop] = typeof node[prop] === "boolean" && value === "" ? true : value;
    } else {
      attr(node, prop, value);
    }
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
  function set_style(node, key, value, important) {
    if (value == null) {
      node.style.removeProperty(key);
    } else {
      node.style.setProperty(key, value, important ? "important" : "");
    }
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
  function onMount(fn) {
    get_current_component().$$.on_mount.push(fn);
  }
  function onDestroy(fn) {
    get_current_component().$$.on_destroy.push(fn);
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

  // node_modules/svelte/src/runtime/internal/each.js
  function ensure_array_like(array_like_or_iterator) {
    return array_like_or_iterator?.length !== void 0 ? array_like_or_iterator : Array.from(array_like_or_iterator);
  }
  function destroy_block(block, lookup) {
    block.d(1);
    lookup.delete(block.key);
  }
  function update_keyed_each(old_blocks, dirty, get_key, dynamic, ctx, list, lookup, node, destroy, create_each_block2, next, get_context) {
    let o = old_blocks.length;
    let n = list.length;
    let i = o;
    const old_indexes = {};
    while (i--) old_indexes[old_blocks[i].key] = i;
    const new_blocks = [];
    const new_lookup = /* @__PURE__ */ new Map();
    const deltas = /* @__PURE__ */ new Map();
    const updates = [];
    i = n;
    while (i--) {
      const child_ctx = get_context(ctx, list, i);
      const key = get_key(child_ctx);
      let block = lookup.get(key);
      if (!block) {
        block = create_each_block2(key, child_ctx);
        block.c();
      } else if (dynamic) {
        updates.push(() => block.p(child_ctx, dirty));
      }
      new_lookup.set(key, new_blocks[i] = block);
      if (key in old_indexes) deltas.set(key, Math.abs(i - old_indexes[key]));
    }
    const will_move = /* @__PURE__ */ new Set();
    const did_move = /* @__PURE__ */ new Set();
    function insert2(block) {
      transition_in(block, 1);
      block.m(node, next);
      lookup.set(block.key, block);
      next = block.first;
      n--;
    }
    while (o && n) {
      const new_block = new_blocks[n - 1];
      const old_block = old_blocks[o - 1];
      const new_key = new_block.key;
      const old_key = old_block.key;
      if (new_block === old_block) {
        next = new_block.first;
        o--;
        n--;
      } else if (!new_lookup.has(old_key)) {
        destroy(old_block, lookup);
        o--;
      } else if (!lookup.has(new_key) || will_move.has(new_key)) {
        insert2(new_block);
      } else if (did_move.has(old_key)) {
        o--;
      } else if (deltas.get(new_key) > deltas.get(old_key)) {
        did_move.add(new_key);
        insert2(new_block);
      } else {
        will_move.add(old_key);
        o--;
      }
    }
    while (o--) {
      const old_block = old_blocks[o];
      if (!new_lookup.has(old_block.key)) destroy(old_block, lookup);
    }
    while (n) insert2(new_blocks[n - 1]);
    run_all(updates);
    return new_blocks;
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

  // chat_thread_shell.svelte.js
  function get_each_context(ctx, list, i) {
    const child_ctx = ctx.slice();
    child_ctx[18] = list[i];
    child_ctx[20] = i;
    return child_ctx;
  }
  function get_each_context_1(ctx, list, i) {
    const child_ctx = ctx.slice();
    child_ctx[21] = list[i];
    return child_ctx;
  }
  function get_each_context_2(ctx, list, i) {
    const child_ctx = ctx.slice();
    child_ctx[24] = list[i];
    return child_ctx;
  }
  function get_each_context_3(ctx, list, i) {
    const child_ctx = ctx.slice();
    child_ctx[27] = list[i];
    return child_ctx;
  }
  function get_each_context_4(ctx, list, i) {
    const child_ctx = ctx.slice();
    child_ctx[30] = list[i];
    return child_ctx;
  }
  function get_each_context_5(ctx, list, i) {
    const child_ctx = ctx.slice();
    child_ctx[33] = list[i];
    return child_ctx;
  }
  function create_if_block_20(ctx) {
    let infring_chat_divider_shell;
    let div;
    let span0;
    let t0;
    let span3;
    let span1;
    let t1_value = (
      /*msg*/
      (ctx[18].notice_icon || "i") + ""
    );
    let t1;
    let t2;
    let span2;
    let t3_value = (
      /*msg*/
      (ctx[18].notice_type === "info" ? "Chat info: " + String(
        /*msg*/
        ctx[18].notice_label || "Info update"
      ) : (
        /*msg*/
        ctx[18].notice_label || "Model switched"
      )) + ""
    );
    let t3;
    let t4;
    let button;
    let t5_value = callStr(
      "noticeActionLabel",
      /*msg*/
      ctx[18]
    ) + "";
    let t5;
    let button_disabled_value;
    let t6;
    let span4;
    let mounted;
    let dispose;
    function click_handler() {
      return (
        /*click_handler*/
        ctx[9](
          /*msg*/
          ctx[18]
        )
      );
    }
    return {
      c() {
        infring_chat_divider_shell = element("infring-chat-divider-shell");
        div = element("div");
        span0 = element("span");
        t0 = space();
        span3 = element("span");
        span1 = element("span");
        t1 = text(t1_value);
        t2 = space();
        span2 = element("span");
        t3 = text(t3_value);
        t4 = space();
        button = element("button");
        t5 = text(t5_value);
        t6 = space();
        span4 = element("span");
        attr(span0, "class", "chat-day-divider-line");
        attr(span0, "aria-hidden", "true");
        attr(span1, "class", "chat-event-info-icon");
        attr(span1, "aria-hidden", "true");
        set_style(
          span1,
          "display",
          /*msg*/
          ctx[18].notice_type === "info" && !callBool(
            "isRenameNotice",
            /*msg*/
            ctx[18]
          ) ? "" : "none"
        );
        attr(button, "class", "chat-notice-action-btn");
        attr(button, "type", "button");
        button.disabled = button_disabled_value = callBool(
          "noticeActionBusy",
          /*msg*/
          ctx[18]
        );
        set_style(button, "display", callBool(
          "noticeActionVisible",
          /*msg*/
          ctx[18]
        ) ? "" : "none");
        attr(span3, "class", "chat-day-divider-label");
        attr(span4, "class", "chat-day-divider-line");
        attr(span4, "aria-hidden", "true");
        attr(div, "class", "chat-day-divider chat-event-divider");
      },
      m(target, anchor) {
        insert(target, infring_chat_divider_shell, anchor);
        append(infring_chat_divider_shell, div);
        append(div, span0);
        append(div, t0);
        append(div, span3);
        append(span3, span1);
        append(span1, t1);
        append(span3, t2);
        append(span3, span2);
        append(span2, t3);
        append(span3, t4);
        append(span3, button);
        append(button, t5);
        append(div, t6);
        append(div, span4);
        if (!mounted) {
          dispose = listen(button, "click", stop_propagation(click_handler));
          mounted = true;
        }
      },
      p(new_ctx, dirty) {
        ctx = new_ctx;
        if (dirty[0] & /*messages*/
        1 && t1_value !== (t1_value = /*msg*/
        (ctx[18].notice_icon || "i") + "")) set_data(t1, t1_value);
        if (dirty[0] & /*messages*/
        1) {
          set_style(
            span1,
            "display",
            /*msg*/
            ctx[18].notice_type === "info" && !callBool(
              "isRenameNotice",
              /*msg*/
              ctx[18]
            ) ? "" : "none"
          );
        }
        if (dirty[0] & /*messages*/
        1 && t3_value !== (t3_value = /*msg*/
        (ctx[18].notice_type === "info" ? "Chat info: " + String(
          /*msg*/
          ctx[18].notice_label || "Info update"
        ) : (
          /*msg*/
          ctx[18].notice_label || "Model switched"
        )) + "")) set_data(t3, t3_value);
        if (dirty[0] & /*messages*/
        1 && t5_value !== (t5_value = callStr(
          "noticeActionLabel",
          /*msg*/
          ctx[18]
        ) + "")) set_data(t5, t5_value);
        if (dirty[0] & /*messages*/
        1 && button_disabled_value !== (button_disabled_value = callBool(
          "noticeActionBusy",
          /*msg*/
          ctx[18]
        ))) {
          button.disabled = button_disabled_value;
        }
        if (dirty[0] & /*messages*/
        1) {
          set_style(button, "display", callBool(
            "noticeActionVisible",
            /*msg*/
            ctx[18]
          ) ? "" : "none");
        }
      },
      d(detaching) {
        if (detaching) {
          detach(infring_chat_divider_shell);
        }
        mounted = false;
        dispose();
      }
    };
  }
  function create_if_block_19(ctx) {
    let infring_chat_divider_shell;
    let div;
    let span0;
    let t0;
    let span1;
    let t1_value = callStr(
      "messageDayLabel",
      /*msg*/
      ctx[18]
    ) + "";
    let t1;
    let t2;
    let span2;
    let div_id_value;
    let div_data_day_value;
    return {
      c() {
        infring_chat_divider_shell = element("infring-chat-divider-shell");
        div = element("div");
        span0 = element("span");
        t0 = space();
        span1 = element("span");
        t1 = text(t1_value);
        t2 = space();
        span2 = element("span");
        attr(span0, "class", "chat-day-divider-line");
        attr(span0, "aria-hidden", "true");
        attr(span1, "class", "chat-day-divider-label");
        attr(span2, "class", "chat-day-divider-line");
        attr(span2, "aria-hidden", "true");
        attr(div, "class", "chat-day-anchor chat-day-divider");
        attr(div, "id", div_id_value = callStr(
          "messageDayDomId",
          /*msg*/
          ctx[18]
        ));
        attr(div, "data-day", div_data_day_value = callStr(
          "messageDayKey",
          /*msg*/
          ctx[18]
        ));
      },
      m(target, anchor) {
        insert(target, infring_chat_divider_shell, anchor);
        append(infring_chat_divider_shell, div);
        append(div, span0);
        append(div, t0);
        append(div, span1);
        append(span1, t1);
        append(div, t2);
        append(div, span2);
      },
      p(ctx2, dirty) {
        if (dirty[0] & /*messages*/
        1 && t1_value !== (t1_value = callStr(
          "messageDayLabel",
          /*msg*/
          ctx2[18]
        ) + "")) set_data(t1, t1_value);
        if (dirty[0] & /*messages*/
        1 && div_id_value !== (div_id_value = callStr(
          "messageDayDomId",
          /*msg*/
          ctx2[18]
        ))) {
          attr(div, "id", div_id_value);
        }
        if (dirty[0] & /*messages*/
        1 && div_data_day_value !== (div_data_day_value = callStr(
          "messageDayKey",
          /*msg*/
          ctx2[18]
        ))) {
          attr(div, "data-day", div_data_day_value);
        }
      },
      d(detaching) {
        if (detaching) {
          detach(infring_chat_divider_shell);
        }
      }
    };
  }
  function create_if_block_18(ctx) {
    let infring_message_terminal_shell;
    let div;
    let span0;
    let t0;
    let span3;
    let span1;
    let t2;
    let span2;
    let t3_value = callStr(
      "terminalToolboxPreview",
      /*msg*/
      ctx[18]
    ) + "";
    let t3;
    let div_class_value;
    let mounted;
    let dispose;
    function click_handler_1() {
      return (
        /*click_handler_1*/
        ctx[10](
          /*msg*/
          ctx[18],
          /*idx*/
          ctx[20]
        )
      );
    }
    function keydown_handler(...args) {
      return (
        /*keydown_handler*/
        ctx[11](
          /*msg*/
          ctx[18],
          /*idx*/
          ctx[20],
          ...args
        )
      );
    }
    return {
      c() {
        infring_message_terminal_shell = element("infring-message-terminal-shell");
        div = element("div");
        span0 = element("span");
        span0.innerHTML = `<svg viewBox="0 0 24 24" focusable="false"><path d="m4 6 6 6-6 6"></path><path d="M12 18h8"></path></svg>`;
        t0 = space();
        span3 = element("span");
        span1 = element("span");
        span1.textContent = "Terminal Output";
        t2 = space();
        span2 = element("span");
        t3 = text(t3_value);
        attr(span0, "class", "terminal-toolbox-icon");
        attr(span0, "aria-hidden", "true");
        attr(span1, "class", "terminal-toolbox-title");
        attr(span2, "class", "terminal-toolbox-preview");
        attr(span3, "class", "terminal-toolbox-copy");
        attr(div, "class", div_class_value = "terminal-toolbox " + callStr(
          "terminalToolboxSideClass",
          /*msg*/
          ctx[18]
        ));
        attr(div, "role", "button");
        attr(div, "tabindex", "0");
        attr(div, "title", "Click to expand full output");
      },
      m(target, anchor) {
        insert(target, infring_message_terminal_shell, anchor);
        append(infring_message_terminal_shell, div);
        append(div, span0);
        append(div, t0);
        append(div, span3);
        append(span3, span1);
        append(span3, t2);
        append(span3, span2);
        append(span2, t3);
        if (!mounted) {
          dispose = [
            listen(div, "click", click_handler_1),
            listen(div, "keydown", keydown_handler)
          ];
          mounted = true;
        }
      },
      p(new_ctx, dirty) {
        ctx = new_ctx;
        if (dirty[0] & /*messages*/
        1 && t3_value !== (t3_value = callStr(
          "terminalToolboxPreview",
          /*msg*/
          ctx[18]
        ) + "")) set_data(t3, t3_value);
        if (dirty[0] & /*messages*/
        1 && div_class_value !== (div_class_value = "terminal-toolbox " + callStr(
          "terminalToolboxSideClass",
          /*msg*/
          ctx[18]
        ))) {
          attr(div, "class", div_class_value);
        }
      },
      d(detaching) {
        if (detaching) {
          detach(infring_message_terminal_shell);
        }
        mounted = false;
        run_all(dispose);
      }
    };
  }
  function create_else_block_1(ctx) {
    let infring_message_placeholder_shell;
    let div;
    let div_style_value;
    let each_value_5 = ensure_array_like(callArr(
      "messagePlaceholderLineIndices",
      /*msg*/
      ctx[18],
      /*idx*/
      ctx[20],
      /*messages*/
      ctx[0]
    ));
    let each_blocks = [];
    for (let i = 0; i < each_value_5.length; i += 1) {
      each_blocks[i] = create_each_block_5(get_each_context_5(ctx, each_value_5, i));
    }
    return {
      c() {
        infring_message_placeholder_shell = element("infring-message-placeholder-shell");
        div = element("div");
        for (let i = 0; i < each_blocks.length; i += 1) {
          each_blocks[i].c();
        }
        attr(div, "class", "message-placeholder-shell message-placeholder-shell-inline");
        attr(div, "style", div_style_value = callStr(
          "messagePlaceholderStyle",
          /*msg*/
          ctx[18],
          /*idx*/
          ctx[20],
          /*messages*/
          ctx[0]
        ));
      },
      m(target, anchor) {
        insert(target, infring_message_placeholder_shell, anchor);
        append(infring_message_placeholder_shell, div);
        for (let i = 0; i < each_blocks.length; i += 1) {
          if (each_blocks[i]) {
            each_blocks[i].m(div, null);
          }
        }
      },
      p(ctx2, dirty) {
        if (dirty[0] & /*messages*/
        1) {
          each_value_5 = ensure_array_like(callArr(
            "messagePlaceholderLineIndices",
            /*msg*/
            ctx2[18],
            /*idx*/
            ctx2[20],
            /*messages*/
            ctx2[0]
          ));
          let i;
          for (i = 0; i < each_value_5.length; i += 1) {
            const child_ctx = get_each_context_5(ctx2, each_value_5, i);
            if (each_blocks[i]) {
              each_blocks[i].p(child_ctx, dirty);
            } else {
              each_blocks[i] = create_each_block_5(child_ctx);
              each_blocks[i].c();
              each_blocks[i].m(div, null);
            }
          }
          for (; i < each_blocks.length; i += 1) {
            each_blocks[i].d(1);
          }
          each_blocks.length = each_value_5.length;
        }
        if (dirty[0] & /*messages*/
        1 && div_style_value !== (div_style_value = callStr(
          "messagePlaceholderStyle",
          /*msg*/
          ctx2[18],
          /*idx*/
          ctx2[20],
          /*messages*/
          ctx2[0]
        ))) {
          attr(div, "style", div_style_value);
        }
      },
      d(detaching) {
        if (detaching) {
          detach(infring_message_placeholder_shell);
        }
        destroy_each(each_blocks, detaching);
      }
    };
  }
  function create_if_block_17(ctx) {
    let infring_chat_bubble_render;
    let infring_chat_bubble_render_typing_value;
    let infring_chat_bubble_render_html_value;
    let infring_chat_bubble_render_plain_value;
    return {
      c() {
        infring_chat_bubble_render = element("infring-chat-bubble-render");
        set_custom_element_data(infring_chat_bubble_render, "typing", infring_chat_bubble_render_typing_value = !!/*msg*/
        ctx[18]._typingVisual ? "1" : "0");
        set_custom_element_data(infring_chat_bubble_render, "html", infring_chat_bubble_render_html_value = callStr(
          "messageBubbleHtml",
          /*msg*/
          ctx[18]
        ));
        set_custom_element_data(infring_chat_bubble_render, "plain", infring_chat_bubble_render_plain_value = String(
          /*msg*/
          ctx[18].text || ""
        ));
      },
      m(target, anchor) {
        insert(target, infring_chat_bubble_render, anchor);
      },
      p(ctx2, dirty) {
        if (dirty[0] & /*messages*/
        1 && infring_chat_bubble_render_typing_value !== (infring_chat_bubble_render_typing_value = !!/*msg*/
        ctx2[18]._typingVisual ? "1" : "0")) {
          set_custom_element_data(infring_chat_bubble_render, "typing", infring_chat_bubble_render_typing_value);
        }
        if (dirty[0] & /*messages*/
        1 && infring_chat_bubble_render_html_value !== (infring_chat_bubble_render_html_value = callStr(
          "messageBubbleHtml",
          /*msg*/
          ctx2[18]
        ))) {
          set_custom_element_data(infring_chat_bubble_render, "html", infring_chat_bubble_render_html_value);
        }
        if (dirty[0] & /*messages*/
        1 && infring_chat_bubble_render_plain_value !== (infring_chat_bubble_render_plain_value = String(
          /*msg*/
          ctx2[18].text || ""
        ))) {
          set_custom_element_data(infring_chat_bubble_render, "plain", infring_chat_bubble_render_plain_value);
        }
      },
      d(detaching) {
        if (detaching) {
          detach(infring_chat_bubble_render);
        }
      }
    };
  }
  function create_each_block_5(ctx) {
    let span;
    let span_class_value;
    return {
      c() {
        span = element("span");
        attr(span, "class", span_class_value = "message-placeholder-line" + /*lineIdx*/
        (ctx[33] === call(
          "messagePlaceholderResolvedLineCount",
          /*msg*/
          ctx[18],
          /*idx*/
          ctx[20],
          /*messages*/
          ctx[0]
        ) - 1 && call(
          "messagePlaceholderResolvedLineCount",
          /*msg*/
          ctx[18],
          /*idx*/
          ctx[20],
          /*messages*/
          ctx[0]
        ) > 1 ? " message-placeholder-line-short" : ""));
      },
      m(target, anchor) {
        insert(target, span, anchor);
      },
      p(ctx2, dirty) {
        if (dirty[0] & /*messages*/
        1 && span_class_value !== (span_class_value = "message-placeholder-line" + /*lineIdx*/
        (ctx2[33] === call(
          "messagePlaceholderResolvedLineCount",
          /*msg*/
          ctx2[18],
          /*idx*/
          ctx2[20],
          /*messages*/
          ctx2[0]
        ) - 1 && call(
          "messagePlaceholderResolvedLineCount",
          /*msg*/
          ctx2[18],
          /*idx*/
          ctx2[20],
          /*messages*/
          ctx2[0]
        ) > 1 ? " message-placeholder-line-short" : ""))) {
          attr(span, "class", span_class_value);
        }
      },
      d(detaching) {
        if (detaching) {
          detach(span);
        }
      }
    };
  }
  function create_each_block_4(key_1, ctx) {
    let a;
    let span0;
    let t0_value = (
      /*chip*/
      ctx[30].label + ""
    );
    let t0;
    let t1;
    let span1;
    let t2_value = (
      /*chip*/
      ctx[30].host + ""
    );
    let t2;
    let t3;
    let a_href_value;
    let a_title_value;
    return {
      key: key_1,
      first: null,
      c() {
        a = element("a");
        span0 = element("span");
        t0 = text(t0_value);
        t1 = space();
        span1 = element("span");
        t2 = text(t2_value);
        t3 = space();
        attr(span0, "class", "message-source-chip-label");
        attr(span1, "class", "message-source-chip-host");
        set_style(
          span1,
          "display",
          /*chip*/
          ctx[30].host ? "" : "none"
        );
        attr(a, "class", "message-source-chip");
        attr(a, "href", a_href_value = /*chip*/
        ctx[30].url);
        attr(a, "target", "_blank");
        attr(a, "rel", "noopener");
        attr(a, "title", a_title_value = /*chip*/
        ctx[30].url);
        this.first = a;
      },
      m(target, anchor) {
        insert(target, a, anchor);
        append(a, span0);
        append(span0, t0);
        append(a, t1);
        append(a, span1);
        append(span1, t2);
        append(a, t3);
      },
      p(new_ctx, dirty) {
        ctx = new_ctx;
        if (dirty[0] & /*messages*/
        1 && t0_value !== (t0_value = /*chip*/
        ctx[30].label + "")) set_data(t0, t0_value);
        if (dirty[0] & /*messages*/
        1 && t2_value !== (t2_value = /*chip*/
        ctx[30].host + "")) set_data(t2, t2_value);
        if (dirty[0] & /*messages*/
        1) {
          set_style(
            span1,
            "display",
            /*chip*/
            ctx[30].host ? "" : "none"
          );
        }
        if (dirty[0] & /*messages*/
        1 && a_href_value !== (a_href_value = /*chip*/
        ctx[30].url)) {
          attr(a, "href", a_href_value);
        }
        if (dirty[0] & /*messages*/
        1 && a_title_value !== (a_title_value = /*chip*/
        ctx[30].url)) {
          attr(a, "title", a_title_value);
        }
      },
      d(detaching) {
        if (detaching) {
          detach(a);
        }
      }
    };
  }
  function create_if_block_16(ctx) {
    let infring_message_progress_shell;
    let div2;
    let div0;
    let span0;
    let t0_value = callObj(
      "messageProgress",
      /*msg*/
      ctx[18]
    ).label + "";
    let t0;
    let t1;
    let span1;
    let t2_value = callObj(
      "messageProgress",
      /*msg*/
      ctx[18]
    ).percent + "%";
    let t2;
    let t3;
    let div1;
    let span2;
    let span2_style_value;
    return {
      c() {
        infring_message_progress_shell = element("infring-message-progress-shell");
        div2 = element("div");
        div0 = element("div");
        span0 = element("span");
        t0 = text(t0_value);
        t1 = space();
        span1 = element("span");
        t2 = text(t2_value);
        t3 = space();
        div1 = element("div");
        span2 = element("span");
        attr(div0, "class", "chat-progress-meta");
        attr(span2, "class", "chat-progress-fill");
        attr(span2, "style", span2_style_value = callStr(
          "progressFillStyle",
          /*msg*/
          ctx[18]
        ));
        attr(div1, "class", "chat-progress-track");
        attr(div2, "class", "chat-progress-wrap");
      },
      m(target, anchor) {
        insert(target, infring_message_progress_shell, anchor);
        append(infring_message_progress_shell, div2);
        append(div2, div0);
        append(div0, span0);
        append(span0, t0);
        append(div0, t1);
        append(div0, span1);
        append(span1, t2);
        append(div2, t3);
        append(div2, div1);
        append(div1, span2);
      },
      p(ctx2, dirty) {
        if (dirty[0] & /*messages*/
        1 && t0_value !== (t0_value = callObj(
          "messageProgress",
          /*msg*/
          ctx2[18]
        ).label + "")) set_data(t0, t0_value);
        if (dirty[0] & /*messages*/
        1 && t2_value !== (t2_value = callObj(
          "messageProgress",
          /*msg*/
          ctx2[18]
        ).percent + "%")) set_data(t2, t2_value);
        if (dirty[0] & /*messages*/
        1 && span2_style_value !== (span2_style_value = callStr(
          "progressFillStyle",
          /*msg*/
          ctx2[18]
        ))) {
          attr(span2, "style", span2_style_value);
        }
      },
      d(detaching) {
        if (detaching) {
          detach(infring_message_progress_shell);
        }
      }
    };
  }
  function create_if_block_15(ctx) {
    let infring_message_artifact_shell;
    let div1;
    let div0;
    let span0;
    let t1;
    let span1;
    let t2_value = (
      /*msg*/
      ctx[18].file_output.path + ""
    );
    let t2;
    let t3;
    let pre;
    let t4_value = (
      /*msg*/
      (ctx[18].file_output.content || "") + ""
    );
    let t4;
    return {
      c() {
        infring_message_artifact_shell = element("infring-message-artifact-shell");
        div1 = element("div");
        div0 = element("div");
        span0 = element("span");
        span0.textContent = "File Output";
        t1 = space();
        span1 = element("span");
        t2 = text(t2_value);
        t3 = space();
        pre = element("pre");
        t4 = text(t4_value);
        attr(span0, "class", "chat-artifact-title");
        attr(span1, "class", "chat-artifact-path");
        attr(div0, "class", "chat-artifact-head");
        attr(pre, "class", "chat-artifact-pre");
        attr(div1, "class", "chat-artifact-card chat-file-output");
      },
      m(target, anchor) {
        insert(target, infring_message_artifact_shell, anchor);
        append(infring_message_artifact_shell, div1);
        append(div1, div0);
        append(div0, span0);
        append(div0, t1);
        append(div0, span1);
        append(span1, t2);
        append(div1, t3);
        append(div1, pre);
        append(pre, t4);
      },
      p(ctx2, dirty) {
        if (dirty[0] & /*messages*/
        1 && t2_value !== (t2_value = /*msg*/
        ctx2[18].file_output.path + "")) set_data(t2, t2_value);
        if (dirty[0] & /*messages*/
        1 && t4_value !== (t4_value = /*msg*/
        (ctx2[18].file_output.content || "") + "")) set_data(t4, t4_value);
      },
      d(detaching) {
        if (detaching) {
          detach(infring_message_artifact_shell);
        }
      }
    };
  }
  function create_if_block_13(ctx) {
    let infring_message_artifact_shell;
    let div1;
    let div0;
    let span0;
    let t1;
    let span1;
    let t2_value = (
      /*msg*/
      ctx[18].folder_output.path + ""
    );
    let t2;
    let t3;
    let pre;
    let t4_value = (
      /*msg*/
      (ctx[18].folder_output.tree || "") + ""
    );
    let t4;
    let t5;
    let if_block = (
      /*msg*/
      ctx[18].folder_output.download_url && create_if_block_14(ctx)
    );
    return {
      c() {
        infring_message_artifact_shell = element("infring-message-artifact-shell");
        div1 = element("div");
        div0 = element("div");
        span0 = element("span");
        span0.textContent = "Folder Output";
        t1 = space();
        span1 = element("span");
        t2 = text(t2_value);
        t3 = space();
        pre = element("pre");
        t4 = text(t4_value);
        t5 = space();
        if (if_block) if_block.c();
        attr(span0, "class", "chat-artifact-title");
        attr(span1, "class", "chat-artifact-path");
        attr(div0, "class", "chat-artifact-head");
        attr(pre, "class", "chat-artifact-pre");
        attr(div1, "class", "chat-artifact-card chat-folder-output");
      },
      m(target, anchor) {
        insert(target, infring_message_artifact_shell, anchor);
        append(infring_message_artifact_shell, div1);
        append(div1, div0);
        append(div0, span0);
        append(div0, t1);
        append(div0, span1);
        append(span1, t2);
        append(div1, t3);
        append(div1, pre);
        append(pre, t4);
        append(div1, t5);
        if (if_block) if_block.m(div1, null);
      },
      p(ctx2, dirty) {
        if (dirty[0] & /*messages*/
        1 && t2_value !== (t2_value = /*msg*/
        ctx2[18].folder_output.path + "")) set_data(t2, t2_value);
        if (dirty[0] & /*messages*/
        1 && t4_value !== (t4_value = /*msg*/
        (ctx2[18].folder_output.tree || "") + "")) set_data(t4, t4_value);
        if (
          /*msg*/
          ctx2[18].folder_output.download_url
        ) {
          if (if_block) {
            if_block.p(ctx2, dirty);
          } else {
            if_block = create_if_block_14(ctx2);
            if_block.c();
            if_block.m(div1, null);
          }
        } else if (if_block) {
          if_block.d(1);
          if_block = null;
        }
      },
      d(detaching) {
        if (detaching) {
          detach(infring_message_artifact_shell);
        }
        if (if_block) if_block.d();
      }
    };
  }
  function create_if_block_14(ctx) {
    let a;
    let t;
    let a_href_value;
    return {
      c() {
        a = element("a");
        t = text("Download archive");
        attr(a, "class", "chat-folder-download-link");
        attr(a, "href", a_href_value = /*msg*/
        ctx[18].folder_output.download_url);
        attr(a, "target", "_blank");
        attr(a, "rel", "noopener");
      },
      m(target, anchor) {
        insert(target, a, anchor);
        append(a, t);
      },
      p(ctx2, dirty) {
        if (dirty[0] & /*messages*/
        1 && a_href_value !== (a_href_value = /*msg*/
        ctx2[18].folder_output.download_url)) {
          attr(a, "href", a_href_value);
        }
      },
      d(detaching) {
        if (detaching) {
          detach(a);
        }
      }
    };
  }
  function create_if_block_12(ctx) {
    let infring_message_media_shell;
    let div;
    let each_blocks = [];
    let each_1_lookup = /* @__PURE__ */ new Map();
    let each_value_3 = ensure_array_like(
      /*msg*/
      ctx[18].images
    );
    const get_key = (ctx2) => (
      /*img*/
      ctx2[27].file_id
    );
    for (let i = 0; i < each_value_3.length; i += 1) {
      let child_ctx = get_each_context_3(ctx, each_value_3, i);
      let key = get_key(child_ctx);
      each_1_lookup.set(key, each_blocks[i] = create_each_block_3(key, child_ctx));
    }
    return {
      c() {
        infring_message_media_shell = element("infring-message-media-shell");
        div = element("div");
        for (let i = 0; i < each_blocks.length; i += 1) {
          each_blocks[i].c();
        }
        set_style(div, "display", "flex");
        set_style(div, "flex-wrap", "wrap");
        set_style(div, "gap", "8px");
        set_style(div, "margin", "8px 0");
      },
      m(target, anchor) {
        insert(target, infring_message_media_shell, anchor);
        append(infring_message_media_shell, div);
        for (let i = 0; i < each_blocks.length; i += 1) {
          if (each_blocks[i]) {
            each_blocks[i].m(div, null);
          }
        }
      },
      p(ctx2, dirty) {
        if (dirty[0] & /*messages*/
        1) {
          each_value_3 = ensure_array_like(
            /*msg*/
            ctx2[18].images
          );
          each_blocks = update_keyed_each(each_blocks, dirty, get_key, 1, ctx2, each_value_3, each_1_lookup, div, destroy_block, create_each_block_3, null, get_each_context_3);
        }
      },
      d(detaching) {
        if (detaching) {
          detach(infring_message_media_shell);
        }
        for (let i = 0; i < each_blocks.length; i += 1) {
          each_blocks[i].d();
        }
      }
    };
  }
  function create_each_block_3(key_1, ctx) {
    let a;
    let img_1;
    let img_1_src_value;
    let img_1_alt_value;
    let t;
    let a_href_value;
    return {
      key: key_1,
      first: null,
      c() {
        a = element("a");
        img_1 = element("img");
        t = space();
        if (!src_url_equal(img_1.src, img_1_src_value = "/api/uploads/" + /*img*/
        ctx[27].file_id)) attr(img_1, "src", img_1_src_value);
        attr(img_1, "alt", img_1_alt_value = /*img*/
        ctx[27].filename || "uploaded image");
        set_style(img_1, "max-width", "320px");
        set_style(img_1, "max-height", "320px");
        set_style(img_1, "border-radius", "8px");
        set_style(img_1, "border", "1px solid var(--border)");
        set_style(img_1, "cursor", "pointer");
        attr(img_1, "loading", "lazy");
        attr(a, "href", a_href_value = "/api/uploads/" + /*img*/
        ctx[27].file_id);
        attr(a, "target", "_blank");
        set_style(a, "display", "block");
        this.first = a;
      },
      m(target, anchor) {
        insert(target, a, anchor);
        append(a, img_1);
        append(a, t);
      },
      p(new_ctx, dirty) {
        ctx = new_ctx;
        if (dirty[0] & /*messages*/
        1 && !src_url_equal(img_1.src, img_1_src_value = "/api/uploads/" + /*img*/
        ctx[27].file_id)) {
          attr(img_1, "src", img_1_src_value);
        }
        if (dirty[0] & /*messages*/
        1 && img_1_alt_value !== (img_1_alt_value = /*img*/
        ctx[27].filename || "uploaded image")) {
          attr(img_1, "alt", img_1_alt_value);
        }
        if (dirty[0] & /*messages*/
        1 && a_href_value !== (a_href_value = "/api/uploads/" + /*img*/
        ctx[27].file_id)) {
          attr(a, "href", a_href_value);
        }
      },
      d(detaching) {
        if (detaching) {
          detach(a);
        }
      }
    };
  }
  function create_else_block(ctx) {
    let span;
    return {
      c() {
        span = element("span");
        span.textContent = "\u2717";
        attr(span, "class", "tool-icon-err");
      },
      m(target, anchor) {
        insert(target, span, anchor);
      },
      d(detaching) {
        if (detaching) {
          detach(span);
        }
      }
    };
  }
  function create_if_block_11(ctx) {
    let span;
    return {
      c() {
        span = element("span");
        span.textContent = "\u2713";
        attr(span, "class", "tool-icon-ok");
      },
      m(target, anchor) {
        insert(target, span, anchor);
      },
      d(detaching) {
        if (detaching) {
          detach(span);
        }
      }
    };
  }
  function create_if_block_10(ctx) {
    let span;
    return {
      c() {
        span = element("span");
        span.innerHTML = `<svg viewBox="0 0 24 24" focusable="false"><path d="M12 3 5 6v6c0 5.2 3.6 8.6 7 10 3.4-1.4 7-4.8 7-10V6l-7-3z"></path></svg>`;
        attr(span, "class", "tool-icon-blocked");
        attr(span, "aria-hidden", "true");
      },
      m(target, anchor) {
        insert(target, span, anchor);
      },
      d(detaching) {
        if (detaching) {
          detach(span);
        }
      }
    };
  }
  function create_if_block_9(ctx) {
    let div;
    return {
      c() {
        div = element("div");
        attr(div, "class", "tool-card-spinner");
      },
      m(target, anchor) {
        insert(target, div, anchor);
      },
      d(detaching) {
        if (detaching) {
          detach(div);
        }
      }
    };
  }
  function create_if_block_8(ctx) {
    let span;
    return {
      c() {
        span = element("span");
        span.innerHTML = `<svg viewBox="0 0 24 24" focusable="false"><path d="M9 3c-2.8 0-5 2.2-5 5 0 .5.1 1 .2 1.4A3.8 3.8 0 0 0 3 12.1C3 14.3 4.7 16 6.9 16H9"></path><path d="M15 3c2.8 0 5 2.2 5 5 0 .5-.1 1-.2 1.4a3.8 3.8 0 0 1 1.2 2.7c0 2.2-1.7 3.9-3.9 3.9H15"></path><path d="M9 3v13M15 3v13"></path><path d="M9 7.2h1.1c.6 0 1 .4 1 1v.5c0 .6.4 1 1 1h.8c.6 0 1 .4 1 1V11"></path><path d="M9 11.8h1.1c.6 0 1 .4 1 1v.4c0 .6.4 1 1 1h.8c.6 0 1 .4 1 1V16"></path></svg>`;
        attr(span, "class", "tool-card-thought-brain");
        attr(span, "aria-hidden", "true");
      },
      m(target, anchor) {
        insert(target, span, anchor);
      },
      d(detaching) {
        if (detaching) {
          detach(span);
        }
      }
    };
  }
  function create_if_block_7(ctx) {
    let div;
    let each_value_2 = ensure_array_like(
      /*tool*/
      ctx[21]._imageUrls || []
    );
    let each_blocks = [];
    for (let i = 0; i < each_value_2.length; i += 1) {
      each_blocks[i] = create_each_block_2(get_each_context_2(ctx, each_value_2, i));
    }
    return {
      c() {
        div = element("div");
        for (let i = 0; i < each_blocks.length; i += 1) {
          each_blocks[i].c();
        }
        set_style(div, "padding", "8px 12px");
        set_style(div, "display", "flex");
        set_style(div, "flex-wrap", "wrap");
        set_style(div, "gap", "8px");
      },
      m(target, anchor) {
        insert(target, div, anchor);
        for (let i = 0; i < each_blocks.length; i += 1) {
          if (each_blocks[i]) {
            each_blocks[i].m(div, null);
          }
        }
      },
      p(ctx2, dirty) {
        if (dirty[0] & /*messages*/
        1) {
          each_value_2 = ensure_array_like(
            /*tool*/
            ctx2[21]._imageUrls || []
          );
          let i;
          for (i = 0; i < each_value_2.length; i += 1) {
            const child_ctx = get_each_context_2(ctx2, each_value_2, i);
            if (each_blocks[i]) {
              each_blocks[i].p(child_ctx, dirty);
            } else {
              each_blocks[i] = create_each_block_2(child_ctx);
              each_blocks[i].c();
              each_blocks[i].m(div, null);
            }
          }
          for (; i < each_blocks.length; i += 1) {
            each_blocks[i].d(1);
          }
          each_blocks.length = each_value_2.length;
        }
      },
      d(detaching) {
        if (detaching) {
          detach(div);
        }
        destroy_each(each_blocks, detaching);
      }
    };
  }
  function create_each_block_2(ctx) {
    let a;
    let img_1;
    let img_1_src_value;
    let t;
    let a_href_value;
    return {
      c() {
        a = element("a");
        img_1 = element("img");
        t = space();
        if (!src_url_equal(img_1.src, img_1_src_value = /*iurl*/
        ctx[24])) attr(img_1, "src", img_1_src_value);
        attr(img_1, "alt", "Generated image");
        set_style(img_1, "max-width", "320px");
        set_style(img_1, "max-height", "320px");
        set_style(img_1, "border-radius", "8px");
        set_style(img_1, "border", "1px solid var(--border)");
        set_style(img_1, "cursor", "pointer");
        attr(img_1, "loading", "lazy");
        attr(a, "href", a_href_value = /*iurl*/
        ctx[24]);
        attr(a, "target", "_blank");
        set_style(a, "display", "block");
      },
      m(target, anchor) {
        insert(target, a, anchor);
        append(a, img_1);
        append(a, t);
      },
      p(ctx2, dirty) {
        if (dirty[0] & /*messages*/
        1 && !src_url_equal(img_1.src, img_1_src_value = /*iurl*/
        ctx2[24])) {
          attr(img_1, "src", img_1_src_value);
        }
        if (dirty[0] & /*messages*/
        1 && a_href_value !== (a_href_value = /*iurl*/
        ctx2[24])) {
          attr(a, "href", a_href_value);
        }
      },
      d(detaching) {
        if (detaching) {
          detach(a);
        }
      }
    };
  }
  function create_if_block_5(ctx) {
    let div1;
    let div0;
    let svg;
    let polygon;
    let path0;
    let path1;
    let t0;
    let span;
    let t1_value = "Audio: " + /*tool*/
    ctx[21]._audioFile.split("/").pop();
    let t1;
    let t2;
    let if_block = (
      /*tool*/
      ctx[21]._audioDuration && create_if_block_6(ctx)
    );
    return {
      c() {
        div1 = element("div");
        div0 = element("div");
        svg = svg_element("svg");
        polygon = svg_element("polygon");
        path0 = svg_element("path");
        path1 = svg_element("path");
        t0 = space();
        span = element("span");
        t1 = text(t1_value);
        t2 = space();
        if (if_block) if_block.c();
        attr(polygon, "points", "11 5 6 9 2 9 2 15 6 15 11 19 11 5");
        attr(path0, "d", "M15.54 8.46a5 5 0 0 1 0 7.07");
        attr(path1, "d", "M19.07 4.93a10 10 0 0 1 0 14.14");
        attr(svg, "width", "14");
        attr(svg, "height", "14");
        attr(svg, "viewBox", "0 0 24 24");
        attr(svg, "fill", "none");
        attr(svg, "stroke", "var(--accent)");
        attr(svg, "stroke-width", "2");
        attr(svg, "stroke-linecap", "round");
        attr(svg, "stroke-linejoin", "round");
        attr(span, "class", "text-xs");
        attr(div0, "class", "audio-player");
        set_style(div1, "padding", "8px 12px");
      },
      m(target, anchor) {
        insert(target, div1, anchor);
        append(div1, div0);
        append(div0, svg);
        append(svg, polygon);
        append(svg, path0);
        append(svg, path1);
        append(div0, t0);
        append(div0, span);
        append(span, t1);
        append(div0, t2);
        if (if_block) if_block.m(div0, null);
      },
      p(ctx2, dirty) {
        if (dirty[0] & /*messages*/
        1 && t1_value !== (t1_value = "Audio: " + /*tool*/
        ctx2[21]._audioFile.split("/").pop())) set_data(t1, t1_value);
        if (
          /*tool*/
          ctx2[21]._audioDuration
        ) {
          if (if_block) {
            if_block.p(ctx2, dirty);
          } else {
            if_block = create_if_block_6(ctx2);
            if_block.c();
            if_block.m(div0, null);
          }
        } else if (if_block) {
          if_block.d(1);
          if_block = null;
        }
      },
      d(detaching) {
        if (detaching) {
          detach(div1);
        }
        if (if_block) if_block.d();
      }
    };
  }
  function create_if_block_6(ctx) {
    let span;
    let t_value = "~" + Math.round(
      /*tool*/
      (ctx[21]._audioDuration || 0) / 1e3
    ) + "s";
    let t;
    return {
      c() {
        span = element("span");
        t = text(t_value);
        attr(span, "class", "text-xs text-dim");
      },
      m(target, anchor) {
        insert(target, span, anchor);
        append(span, t);
      },
      p(ctx2, dirty) {
        if (dirty[0] & /*messages*/
        1 && t_value !== (t_value = "~" + Math.round(
          /*tool*/
          (ctx2[21]._audioDuration || 0) / 1e3
        ) + "s")) set_data(t, t_value);
      },
      d(detaching) {
        if (detaching) {
          detach(span);
        }
      }
    };
  }
  function create_if_block_1(ctx) {
    let div;
    let t;
    let if_block0 = (
      /*tool*/
      ctx[21].input && create_if_block_4(ctx)
    );
    let if_block1 = (
      /*tool*/
      ctx[21].result && create_if_block_2(ctx)
    );
    return {
      c() {
        div = element("div");
        if (if_block0) if_block0.c();
        t = space();
        if (if_block1) if_block1.c();
        attr(div, "class", "tool-card-body");
      },
      m(target, anchor) {
        insert(target, div, anchor);
        if (if_block0) if_block0.m(div, null);
        append(div, t);
        if (if_block1) if_block1.m(div, null);
      },
      p(ctx2, dirty) {
        if (
          /*tool*/
          ctx2[21].input
        ) {
          if (if_block0) {
            if_block0.p(ctx2, dirty);
          } else {
            if_block0 = create_if_block_4(ctx2);
            if_block0.c();
            if_block0.m(div, t);
          }
        } else if (if_block0) {
          if_block0.d(1);
          if_block0 = null;
        }
        if (
          /*tool*/
          ctx2[21].result
        ) {
          if (if_block1) {
            if_block1.p(ctx2, dirty);
          } else {
            if_block1 = create_if_block_2(ctx2);
            if_block1.c();
            if_block1.m(div, null);
          }
        } else if (if_block1) {
          if_block1.d(1);
          if_block1 = null;
        }
      },
      d(detaching) {
        if (detaching) {
          detach(div);
        }
        if (if_block0) if_block0.d();
        if (if_block1) if_block1.d();
      }
    };
  }
  function create_if_block_4(ctx) {
    let div1;
    let div0;
    let t1;
    let pre;
    let t2_value = callStr(
      "formatToolJson",
      /*tool*/
      ctx[21].input
    ) + "";
    let t2;
    return {
      c() {
        div1 = element("div");
        div0 = element("div");
        div0.textContent = "Input";
        t1 = space();
        pre = element("pre");
        t2 = text(t2_value);
        attr(div0, "class", "tool-section-label");
        attr(pre, "class", "tool-pre");
        set_style(div1, "margin-bottom", "6px");
      },
      m(target, anchor) {
        insert(target, div1, anchor);
        append(div1, div0);
        append(div1, t1);
        append(div1, pre);
        append(pre, t2);
      },
      p(ctx2, dirty) {
        if (dirty[0] & /*messages*/
        1 && t2_value !== (t2_value = callStr(
          "formatToolJson",
          /*tool*/
          ctx2[21].input
        ) + "")) set_data(t2, t2_value);
      },
      d(detaching) {
        if (detaching) {
          detach(div1);
        }
      }
    };
  }
  function create_if_block_2(ctx) {
    let div1;
    let div0;
    let t0;
    let t1;
    let pre;
    let t2_value = callStr(
      "formatToolJson",
      /*tool*/
      ctx[21].result
    ) + "";
    let t2;
    let pre_class_value;
    let if_block = (
      /*tool*/
      ctx[21].result && /*tool*/
      ctx[21].result.length > 200 && create_if_block_3(ctx)
    );
    return {
      c() {
        div1 = element("div");
        div0 = element("div");
        t0 = text("Result ");
        if (if_block) if_block.c();
        t1 = space();
        pre = element("pre");
        t2 = text(t2_value);
        attr(div0, "class", "tool-section-label");
        attr(pre, "class", pre_class_value = "tool-pre" + /*tool*/
        (ctx[21].is_error ? " tool-pre-error" : !/*tool*/
        ctx[21].is_error && /*tool*/
        ctx[21].result && /*tool*/
        ctx[21].result.length < 100 ? " tool-pre-short" : !/*tool*/
        ctx[21].is_error && /*tool*/
        ctx[21].result && /*tool*/
        ctx[21].result.length < 500 ? " tool-pre-medium" : ""));
      },
      m(target, anchor) {
        insert(target, div1, anchor);
        append(div1, div0);
        append(div0, t0);
        if (if_block) if_block.m(div0, null);
        append(div1, t1);
        append(div1, pre);
        append(pre, t2);
      },
      p(ctx2, dirty) {
        if (
          /*tool*/
          ctx2[21].result && /*tool*/
          ctx2[21].result.length > 200
        ) {
          if (if_block) {
            if_block.p(ctx2, dirty);
          } else {
            if_block = create_if_block_3(ctx2);
            if_block.c();
            if_block.m(div0, null);
          }
        } else if (if_block) {
          if_block.d(1);
          if_block = null;
        }
        if (dirty[0] & /*messages*/
        1 && t2_value !== (t2_value = callStr(
          "formatToolJson",
          /*tool*/
          ctx2[21].result
        ) + "")) set_data(t2, t2_value);
        if (dirty[0] & /*messages*/
        1 && pre_class_value !== (pre_class_value = "tool-pre" + /*tool*/
        (ctx2[21].is_error ? " tool-pre-error" : !/*tool*/
        ctx2[21].is_error && /*tool*/
        ctx2[21].result && /*tool*/
        ctx2[21].result.length < 100 ? " tool-pre-short" : !/*tool*/
        ctx2[21].is_error && /*tool*/
        ctx2[21].result && /*tool*/
        ctx2[21].result.length < 500 ? " tool-pre-medium" : ""))) {
          attr(pre, "class", pre_class_value);
        }
      },
      d(detaching) {
        if (detaching) {
          detach(div1);
        }
        if (if_block) if_block.d();
      }
    };
  }
  function create_if_block_3(ctx) {
    let span;
    let t0;
    let t1_value = (
      /*tool*/
      ctx[21].result.length + ""
    );
    let t1;
    let t2;
    return {
      c() {
        span = element("span");
        t0 = text("(");
        t1 = text(t1_value);
        t2 = text(" chars)");
        attr(span, "class", "text-xs text-muted");
      },
      m(target, anchor) {
        insert(target, span, anchor);
        append(span, t0);
        append(span, t1);
        append(span, t2);
      },
      p(ctx2, dirty) {
        if (dirty[0] & /*messages*/
        1 && t1_value !== (t1_value = /*tool*/
        ctx2[21].result.length + "")) set_data(t1, t1_value);
      },
      d(detaching) {
        if (detaching) {
          detach(span);
        }
      }
    };
  }
  function create_each_block_1(key_1, ctx) {
    let div1;
    let div0;
    let show_if;
    let show_if_1;
    let t0;
    let span0;
    let raw_value = callStr(
      "toolIcon",
      /*tool*/
      ctx[21].name
    ) + "";
    let t1;
    let span1;
    let t2_value = (callBool(
      "isThoughtTool",
      /*tool*/
      ctx[21]
    ) ? callStr(
      "thoughtToolLabel",
      /*tool*/
      ctx[21]
    ) : callStr(
      "toolDisplayName",
      /*tool*/
      ctx[21]
    )) + "";
    let t2;
    let t3;
    let span2;
    let t4_value = callStr(
      "toolStatusText",
      /*tool*/
      ctx[21]
    ) + "";
    let t4;
    let span2_class_value;
    let t5;
    let span3;
    let t6_value = (
      /*tool*/
      ctx[21].expanded ? "\u25BE" : "\u25B8"
    );
    let t6;
    let span3_style_value;
    let t7;
    let t8;
    let t9;
    let t10;
    let div1_class_value;
    let div1_data_tool_value;
    let mounted;
    let dispose;
    function select_block_type_1(ctx2, dirty) {
      if (dirty[0] & /*messages*/
      1) show_if = null;
      if (dirty[0] & /*messages*/
      1) show_if_1 = null;
      if (show_if == null) show_if = !!callBool(
        "isThoughtTool",
        /*tool*/
        ctx2[21]
      );
      if (show_if) return create_if_block_8;
      if (
        /*tool*/
        ctx2[21].running
      ) return create_if_block_9;
      if (show_if_1 == null) show_if_1 = !!callBool(
        "isBlockedTool",
        /*tool*/
        ctx2[21]
      );
      if (show_if_1) return create_if_block_10;
      if (!/*tool*/
      ctx2[21].is_error) return create_if_block_11;
      return create_else_block;
    }
    let current_block_type = select_block_type_1(ctx, [-1, -1]);
    let if_block0 = current_block_type(ctx);
    function click_handler_2() {
      return (
        /*click_handler_2*/
        ctx[12](
          /*tool*/
          ctx[21]
        )
      );
    }
    let if_block1 = (
      /*tool*/
      ctx[21]._imageUrls && /*tool*/
      ctx[21]._imageUrls.length && create_if_block_7(ctx)
    );
    let if_block2 = (
      /*tool*/
      ctx[21]._audioFile && create_if_block_5(ctx)
    );
    let if_block3 = (
      /*tool*/
      ctx[21].expanded && create_if_block_1(ctx)
    );
    return {
      key: key_1,
      first: null,
      c() {
        div1 = element("div");
        div0 = element("div");
        if_block0.c();
        t0 = space();
        span0 = element("span");
        t1 = space();
        span1 = element("span");
        t2 = text(t2_value);
        t3 = space();
        span2 = element("span");
        t4 = text(t4_value);
        t5 = space();
        span3 = element("span");
        t6 = text(t6_value);
        t7 = space();
        if (if_block1) if_block1.c();
        t8 = space();
        if (if_block2) if_block2.c();
        t9 = space();
        if (if_block3) if_block3.c();
        t10 = space();
        attr(span0, "class", "tool-card-icon");
        set_style(span0, "display", callBool(
          "isThoughtTool",
          /*tool*/
          ctx[21]
        ) ? "none" : "");
        attr(span1, "class", "tool-card-name");
        attr(span2, "class", span2_class_value = "text-xs" + (callBool(
          "isBlockedTool",
          /*tool*/
          ctx[21]
        ) ? " tool-status-blocked" : callBool(
          "isToolSuccessful",
          /*tool*/
          ctx[21]
        ) ? " tool-status-success" : " text-dim"));
        set_style(span2, "margin-left", "auto");
        set_style(span2, "display", callBool(
          "isThoughtTool",
          /*tool*/
          ctx[21]
        ) ? "none" : "");
        attr(span3, "class", "tool-expand-chevron");
        attr(span3, "style", span3_style_value = callBool(
          "isThoughtTool",
          /*tool*/
          ctx[21]
        ) ? "margin-left:auto" : "");
        attr(div0, "class", "tool-card-header");
        attr(div1, "class", div1_class_value = "tool-card" + /*tool*/
        (ctx[21].is_error && !callBool(
          "isBlockedTool",
          /*tool*/
          ctx[21]
        ) ? " tool-card-error" : "") + (callBool(
          "isBlockedTool",
          /*tool*/
          ctx[21]
        ) ? " tool-card-blocked" : "") + (callBool(
          "isToolSuccessful",
          /*tool*/
          ctx[21]
        ) ? " tool-card-success" : "") + (callBool(
          "isThoughtTool",
          /*tool*/
          ctx[21]
        ) ? " tool-card-thought" : ""));
        attr(div1, "data-tool", div1_data_tool_value = /*tool*/
        ctx[21].name);
        this.first = div1;
      },
      m(target, anchor) {
        insert(target, div1, anchor);
        append(div1, div0);
        if_block0.m(div0, null);
        append(div0, t0);
        append(div0, span0);
        span0.innerHTML = raw_value;
        append(div0, t1);
        append(div0, span1);
        append(span1, t2);
        append(div0, t3);
        append(div0, span2);
        append(span2, t4);
        append(div0, t5);
        append(div0, span3);
        append(span3, t6);
        append(div1, t7);
        if (if_block1) if_block1.m(div1, null);
        append(div1, t8);
        if (if_block2) if_block2.m(div1, null);
        append(div1, t9);
        if (if_block3) if_block3.m(div1, null);
        append(div1, t10);
        if (!mounted) {
          dispose = listen(div0, "click", click_handler_2);
          mounted = true;
        }
      },
      p(new_ctx, dirty) {
        ctx = new_ctx;
        if (current_block_type !== (current_block_type = select_block_type_1(ctx, dirty))) {
          if_block0.d(1);
          if_block0 = current_block_type(ctx);
          if (if_block0) {
            if_block0.c();
            if_block0.m(div0, t0);
          }
        }
        if (dirty[0] & /*messages*/
        1 && raw_value !== (raw_value = callStr(
          "toolIcon",
          /*tool*/
          ctx[21].name
        ) + "")) span0.innerHTML = raw_value;
        ;
        if (dirty[0] & /*messages*/
        1) {
          set_style(span0, "display", callBool(
            "isThoughtTool",
            /*tool*/
            ctx[21]
          ) ? "none" : "");
        }
        if (dirty[0] & /*messages*/
        1 && t2_value !== (t2_value = (callBool(
          "isThoughtTool",
          /*tool*/
          ctx[21]
        ) ? callStr(
          "thoughtToolLabel",
          /*tool*/
          ctx[21]
        ) : callStr(
          "toolDisplayName",
          /*tool*/
          ctx[21]
        )) + "")) set_data(t2, t2_value);
        if (dirty[0] & /*messages*/
        1 && t4_value !== (t4_value = callStr(
          "toolStatusText",
          /*tool*/
          ctx[21]
        ) + "")) set_data(t4, t4_value);
        if (dirty[0] & /*messages*/
        1 && span2_class_value !== (span2_class_value = "text-xs" + (callBool(
          "isBlockedTool",
          /*tool*/
          ctx[21]
        ) ? " tool-status-blocked" : callBool(
          "isToolSuccessful",
          /*tool*/
          ctx[21]
        ) ? " tool-status-success" : " text-dim"))) {
          attr(span2, "class", span2_class_value);
        }
        if (dirty[0] & /*messages*/
        1) {
          set_style(span2, "display", callBool(
            "isThoughtTool",
            /*tool*/
            ctx[21]
          ) ? "none" : "");
        }
        if (dirty[0] & /*messages*/
        1 && t6_value !== (t6_value = /*tool*/
        ctx[21].expanded ? "\u25BE" : "\u25B8")) set_data(t6, t6_value);
        if (dirty[0] & /*messages*/
        1 && span3_style_value !== (span3_style_value = callBool(
          "isThoughtTool",
          /*tool*/
          ctx[21]
        ) ? "margin-left:auto" : "")) {
          attr(span3, "style", span3_style_value);
        }
        if (
          /*tool*/
          ctx[21]._imageUrls && /*tool*/
          ctx[21]._imageUrls.length
        ) {
          if (if_block1) {
            if_block1.p(ctx, dirty);
          } else {
            if_block1 = create_if_block_7(ctx);
            if_block1.c();
            if_block1.m(div1, t8);
          }
        } else if (if_block1) {
          if_block1.d(1);
          if_block1 = null;
        }
        if (
          /*tool*/
          ctx[21]._audioFile
        ) {
          if (if_block2) {
            if_block2.p(ctx, dirty);
          } else {
            if_block2 = create_if_block_5(ctx);
            if_block2.c();
            if_block2.m(div1, t9);
          }
        } else if (if_block2) {
          if_block2.d(1);
          if_block2 = null;
        }
        if (
          /*tool*/
          ctx[21].expanded
        ) {
          if (if_block3) {
            if_block3.p(ctx, dirty);
          } else {
            if_block3 = create_if_block_1(ctx);
            if_block3.c();
            if_block3.m(div1, t10);
          }
        } else if (if_block3) {
          if_block3.d(1);
          if_block3 = null;
        }
        if (dirty[0] & /*messages*/
        1 && div1_class_value !== (div1_class_value = "tool-card" + /*tool*/
        (ctx[21].is_error && !callBool(
          "isBlockedTool",
          /*tool*/
          ctx[21]
        ) ? " tool-card-error" : "") + (callBool(
          "isBlockedTool",
          /*tool*/
          ctx[21]
        ) ? " tool-card-blocked" : "") + (callBool(
          "isToolSuccessful",
          /*tool*/
          ctx[21]
        ) ? " tool-card-success" : "") + (callBool(
          "isThoughtTool",
          /*tool*/
          ctx[21]
        ) ? " tool-card-thought" : ""))) {
          attr(div1, "class", div1_class_value);
        }
        if (dirty[0] & /*messages*/
        1 && div1_data_tool_value !== (div1_data_tool_value = /*tool*/
        ctx[21].name)) {
          attr(div1, "data-tool", div1_data_tool_value);
        }
      },
      d(detaching) {
        if (detaching) {
          detach(div1);
        }
        if_block0.d();
        if (if_block1) if_block1.d();
        if (if_block2) if_block2.d();
        if (if_block3) if_block3.d();
        mounted = false;
        dispose();
      }
    };
  }
  function create_each_block(key_1, ctx) {
    let div9;
    let t0;
    let show_if_6 = callBool(
      "isNewMessageDay",
      /*messages*/
      ctx[0],
      /*idx*/
      ctx[20]
    );
    let t1;
    let infring_chat_stream_shell;
    let div0;
    let t3;
    let div8;
    let div1;
    let span2;
    let span3;
    let t5_value = callStr(
      "messageTitleLabel",
      /*msg*/
      ctx[18]
    ) + "";
    let t5;
    let span4;
    let div1_class_value;
    let t7;
    let div4;
    let span8;
    let t10;
    let div2;
    let em;
    let t11_value = callStr(
      "thinkingBubbleLineText",
      /*msg*/
      ctx[18]
    ) + "";
    let t11;
    let em_data_shimmer_text_value;
    let t12;
    let div3;
    let t13;
    let show_if_5 = (
      /*msg*/
      ctx[18].terminal && callBool(
        "terminalMessageCollapsed",
        /*msg*/
        ctx[18],
        /*idx*/
        ctx[20],
        /*messages*/
        ctx[0]
      )
    );
    let t14;
    let div5;
    let show_if_4;
    let div5_class_value;
    let t15;
    let infring_message_context_shell;
    let div6;
    let each_blocks_1 = [];
    let each0_lookup = /* @__PURE__ */ new Map();
    let t16;
    let div7;
    let span12;
    let t17_value = callObj(
      "messageToolTraceSummary",
      /*msg*/
      ctx[18]
    ).label + "";
    let t17;
    let t18;
    let span13;
    let t19_value = callObj(
      "messageToolTraceSummary",
      /*msg*/
      ctx[18]
    ).detail + "";
    let t19;
    let t20;
    let show_if_3 = callBool(
      "shouldRenderMessageContent",
      /*msg*/
      ctx[18],
      /*idx*/
      ctx[20],
      /*messages*/
      ctx[0]
    ) && call(
      "messageProgress",
      /*msg*/
      ctx[18]
    );
    let t21;
    let show_if_2 = callBool(
      "shouldRenderMessageContent",
      /*msg*/
      ctx[18],
      /*idx*/
      ctx[20],
      /*messages*/
      ctx[0]
    ) && /*msg*/
    ctx[18].file_output && /*msg*/
    ctx[18].file_output.path;
    let t22;
    let show_if_1 = callBool(
      "shouldRenderMessageContent",
      /*msg*/
      ctx[18],
      /*idx*/
      ctx[20],
      /*messages*/
      ctx[0]
    ) && /*msg*/
    ctx[18].folder_output && /*msg*/
    ctx[18].folder_output.path;
    let t23;
    let show_if = callBool(
      "shouldRenderMessageContent",
      /*msg*/
      ctx[18],
      /*idx*/
      ctx[20],
      /*messages*/
      ctx[0]
    ) && /*msg*/
    ctx[18].images && /*msg*/
    ctx[18].images.length;
    let t24;
    let infring_tool_card_stack_shell;
    let each_blocks = [];
    let each1_lookup = /* @__PURE__ */ new Map();
    let t25;
    let infring_message_meta_shell;
    let infring_message_meta_shell_state_value;
    let infring_chat_stream_shell_class_value;
    let infring_chat_stream_shell_data_message_dom_id_value;
    let infring_chat_stream_shell_data_origin_kind_value;
    let infring_chat_stream_shell_role_value;
    let infring_chat_stream_shell_grouped_value;
    let infring_chat_stream_shell_streaming_value;
    let infring_chat_stream_shell_thinking_value;
    let infring_chat_stream_shell_hovered_value;
    let div9_id_value;
    let div9_data_msg_idx_value;
    let mounted;
    let dispose;
    let if_block0 = (
      /*msg*/
      ctx[18].is_notice && create_if_block_20(ctx)
    );
    let if_block1 = show_if_6 && create_if_block_19(ctx);
    let if_block2 = show_if_5 && create_if_block_18(ctx);
    function select_block_type(ctx2, dirty) {
      if (dirty[0] & /*messages*/
      1) show_if_4 = null;
      if (show_if_4 == null) show_if_4 = !!callBool(
        "shouldRenderMessageContent",
        /*msg*/
        ctx2[18],
        /*idx*/
        ctx2[20],
        /*messages*/
        ctx2[0]
      );
      if (show_if_4) return create_if_block_17;
      return create_else_block_1;
    }
    let current_block_type = select_block_type(ctx, [-1, -1]);
    let if_block3 = current_block_type(ctx);
    let each_value_4 = ensure_array_like(callArr(
      "messageSourceChips",
      /*msg*/
      ctx[18]
    ));
    const get_key = (ctx2) => (
      /*chip*/
      ctx2[30].id
    );
    for (let i = 0; i < each_value_4.length; i += 1) {
      let child_ctx = get_each_context_4(ctx, each_value_4, i);
      let key = get_key(child_ctx);
      each0_lookup.set(key, each_blocks_1[i] = create_each_block_4(key, child_ctx));
    }
    let if_block4 = show_if_3 && create_if_block_16(ctx);
    let if_block5 = show_if_2 && create_if_block_15(ctx);
    let if_block6 = show_if_1 && create_if_block_13(ctx);
    let if_block7 = show_if && create_if_block_12(ctx);
    let each_value_1 = ensure_array_like(callBool(
      "shouldRenderMessageContent",
      /*msg*/
      ctx[18],
      /*idx*/
      ctx[20],
      /*messages*/
      ctx[0]
    ) ? (
      /*msg*/
      ctx[18].tools || []
    ) : []);
    const get_key_1 = (ctx2) => (
      /*tool*/
      ctx2[21].id
    );
    for (let i = 0; i < each_value_1.length; i += 1) {
      let child_ctx = get_each_context_1(ctx, each_value_1, i);
      let key = get_key_1(child_ctx);
      each1_lookup.set(key, each_blocks[i] = create_each_block_1(key, child_ctx));
    }
    function message_meta_action_handler(...args) {
      return (
        /*message_meta_action_handler*/
        ctx[13](
          /*msg*/
          ctx[18],
          /*idx*/
          ctx[20],
          ...args
        )
      );
    }
    function mouseenter_handler() {
      return (
        /*mouseenter_handler*/
        ctx[14](
          /*msg*/
          ctx[18],
          /*idx*/
          ctx[20]
        )
      );
    }
    return {
      key: key_1,
      first: null,
      c() {
        div9 = element("div");
        if (if_block0) if_block0.c();
        t0 = space();
        if (if_block1) if_block1.c();
        t1 = space();
        infring_chat_stream_shell = element("infring-chat-stream-shell");
        div0 = element("div");
        div0.innerHTML = `<span class="agent-mark infring-logo infring-logo--agent-default" aria-hidden="true"><span class="infring-logo-glyph" aria-hidden="true">\u221E</span></span>`;
        t3 = space();
        div8 = element("div");
        div1 = element("div");
        span2 = element("span");
        span2.textContent = "[";
        span3 = element("span");
        t5 = text(t5_value);
        span4 = element("span");
        span4.textContent = "]";
        t7 = space();
        div4 = element("div");
        span8 = element("span");
        span8.innerHTML = `<span class="thinking-orb-link-dot thinking-orb-link-dot-1"></span> <span class="thinking-orb-link-dot thinking-orb-link-dot-2"></span> <span class="thinking-orb-link-dot thinking-orb-link-dot-3"></span>`;
        t10 = space();
        div2 = element("div");
        em = element("em");
        t11 = text(t11_value);
        t12 = space();
        div3 = element("div");
        div3.innerHTML = `<span></span><span></span><span></span>`;
        t13 = space();
        if (if_block2) if_block2.c();
        t14 = space();
        div5 = element("div");
        if_block3.c();
        t15 = space();
        infring_message_context_shell = element("infring-message-context-shell");
        div6 = element("div");
        for (let i = 0; i < each_blocks_1.length; i += 1) {
          each_blocks_1[i].c();
        }
        t16 = space();
        div7 = element("div");
        span12 = element("span");
        t17 = text(t17_value);
        t18 = space();
        span13 = element("span");
        t19 = text(t19_value);
        t20 = space();
        if (if_block4) if_block4.c();
        t21 = space();
        if (if_block5) if_block5.c();
        t22 = space();
        if (if_block6) if_block6.c();
        t23 = space();
        if (if_block7) if_block7.c();
        t24 = space();
        infring_tool_card_stack_shell = element("infring-tool-card-stack-shell");
        for (let i = 0; i < each_blocks.length; i += 1) {
          each_blocks[i].c();
        }
        t25 = space();
        infring_message_meta_shell = element("infring-message-meta-shell");
        attr(div0, "class", "message-avatar");
        set_style(div0, "display", showAvatar(
          /*msg*/
          ctx[18]
        ) ? "" : "none");
        attr(span2, "class", "message-agent-name-bracket");
        attr(span2, "aria-hidden", "true");
        attr(span3, "class", "message-agent-name-label");
        attr(span4, "class", "message-agent-name-bracket");
        attr(span4, "aria-hidden", "true");
        attr(div1, "class", div1_class_value = "message-agent-name " + callStr(
          "messageTitleClass",
          /*msg*/
          ctx[18]
        ) + /*msg*/
        (ctx[18].terminal ? " terminal-actor-label" : ""));
        set_style(div1, "display", callBool(
          "showMessageTitle",
          /*msg*/
          ctx[18],
          /*idx*/
          ctx[20],
          /*messages*/
          ctx[0]
        ) ? "" : "none");
        attr(span8, "class", "thinking-orb-link");
        attr(span8, "aria-hidden", "true");
        attr(em, "class", "thinking-shimmer-text");
        attr(em, "data-shimmer-text", em_data_shimmer_text_value = callStr(
          "thinkingBubbleLineText",
          /*msg*/
          ctx[18]
        ));
        attr(div2, "class", "thinking-inline-text");
        attr(div3, "class", "typing-dots");
        attr(div4, "class", "message-bubble message-bubble-thinking");
        set_style(
          div4,
          "display",
          /*msg*/
          ctx[18].thinking ? "" : "none"
        );
        attr(div5, "class", div5_class_value = "message-bubble" + bubbleClass(
          /*msg*/
          ctx[18]
        ));
        set_style(
          div5,
          "display",
          /*bubbleVisible*/
          ctx[8](
            /*msg*/
            ctx[18],
            /*idx*/
            ctx[20]
          ) ? "" : "none"
        );
        attr(div6, "class", "message-source-chips");
        set_style(div6, "display", callBool(
          "shouldRenderMessageContent",
          /*msg*/
          ctx[18],
          /*idx*/
          ctx[20],
          /*messages*/
          ctx[0]
        ) && callBool(
          "messageHasSourceChips",
          /*msg*/
          ctx[18]
        ) ? "" : "none");
        attr(span12, "class", "message-tool-trace-label");
        attr(span13, "class", "message-tool-trace-detail");
        attr(div7, "class", "message-tool-trace-summary");
        set_style(div7, "display", callBool(
          "shouldRenderMessageContent",
          /*msg*/
          ctx[18],
          /*idx*/
          ctx[20],
          /*messages*/
          ctx[0]
        ) && callObj(
          "messageToolTraceSummary",
          /*msg*/
          ctx[18]
        ).visible ? "" : "none");
        set_custom_element_data(infring_message_meta_shell, "state", infring_message_meta_shell_state_value = callStr(
          "messageMetadataShellState",
          /*msg*/
          ctx[18],
          /*idx*/
          ctx[20],
          /*messages*/
          ctx[0]
        ));
        attr(div8, "class", "message-body");
        set_custom_element_data(infring_chat_stream_shell, "class", infring_chat_stream_shell_class_value = "message " + /*msgClass*/
        ctx[7](
          /*msg*/
          ctx[18],
          /*idx*/
          ctx[20]
        ));
        set_custom_element_data(infring_chat_stream_shell, "data-message-dom-id", infring_chat_stream_shell_data_message_dom_id_value = callStr(
          "messageDomId",
          /*msg*/
          ctx[18],
          /*idx*/
          ctx[20]
        ));
        set_custom_element_data(infring_chat_stream_shell, "data-origin-kind", infring_chat_stream_shell_data_origin_kind_value = callStr(
          "messageOriginKind",
          /*msg*/
          ctx[18]
        ));
        set_custom_element_data(infring_chat_stream_shell, "role", infring_chat_stream_shell_role_value = /*msg*/
        ctx[18].role || "");
        set_custom_element_data(infring_chat_stream_shell, "grouped", infring_chat_stream_shell_grouped_value = callBool(
          "isGrouped",
          /*idx*/
          ctx[20],
          /*messages*/
          ctx[0]
        ) ? "true" : null);
        set_custom_element_data(infring_chat_stream_shell, "streaming", infring_chat_stream_shell_streaming_value = /*msg*/
        ctx[18].streaming ? "true" : null);
        set_custom_element_data(infring_chat_stream_shell, "thinking", infring_chat_stream_shell_thinking_value = /*msg*/
        ctx[18].thinking ? "true" : null);
        set_custom_element_data(infring_chat_stream_shell, "hovered", infring_chat_stream_shell_hovered_value = /*hoveredIdx*/
        ctx[1] === /*idx*/
        ctx[20] ? "true" : null);
        set_style(
          infring_chat_stream_shell,
          "display",
          /*msg*/
          ctx[18].is_notice ? "none" : ""
        );
        attr(div9, "class", "chat-message-block");
        attr(div9, "id", div9_id_value = callStr(
          "messageDomId",
          /*msg*/
          ctx[18],
          /*idx*/
          ctx[20]
        ));
        attr(div9, "data-msg-idx", div9_data_msg_idx_value = /*idx*/
        ctx[20]);
        this.first = div9;
      },
      m(target, anchor) {
        insert(target, div9, anchor);
        if (if_block0) if_block0.m(div9, null);
        append(div9, t0);
        if (if_block1) if_block1.m(div9, null);
        append(div9, t1);
        append(div9, infring_chat_stream_shell);
        append(infring_chat_stream_shell, div0);
        append(infring_chat_stream_shell, t3);
        append(infring_chat_stream_shell, div8);
        append(div8, div1);
        append(div1, span2);
        append(div1, span3);
        append(span3, t5);
        append(div1, span4);
        append(div8, t7);
        append(div8, div4);
        append(div4, span8);
        append(div4, t10);
        append(div4, div2);
        append(div2, em);
        append(em, t11);
        append(div4, t12);
        append(div4, div3);
        append(div8, t13);
        if (if_block2) if_block2.m(div8, null);
        append(div8, t14);
        append(div8, div5);
        if_block3.m(div5, null);
        append(div8, t15);
        append(div8, infring_message_context_shell);
        append(infring_message_context_shell, div6);
        for (let i = 0; i < each_blocks_1.length; i += 1) {
          if (each_blocks_1[i]) {
            each_blocks_1[i].m(div6, null);
          }
        }
        append(infring_message_context_shell, t16);
        append(infring_message_context_shell, div7);
        append(div7, span12);
        append(span12, t17);
        append(div7, t18);
        append(div7, span13);
        append(span13, t19);
        append(div8, t20);
        if (if_block4) if_block4.m(div8, null);
        append(div8, t21);
        if (if_block5) if_block5.m(div8, null);
        append(div8, t22);
        if (if_block6) if_block6.m(div8, null);
        append(div8, t23);
        if (if_block7) if_block7.m(div8, null);
        append(div8, t24);
        append(div8, infring_tool_card_stack_shell);
        for (let i = 0; i < each_blocks.length; i += 1) {
          if (each_blocks[i]) {
            each_blocks[i].m(infring_tool_card_stack_shell, null);
          }
        }
        append(div8, t25);
        append(div8, infring_message_meta_shell);
        if (!mounted) {
          dispose = [
            listen(infring_message_meta_shell, "message-meta-action", message_meta_action_handler),
            listen(infring_chat_stream_shell, "mouseenter", mouseenter_handler),
            listen(
              infring_chat_stream_shell,
              "mouseleave",
              /*mouseleave_handler*/
              ctx[15]
            )
          ];
          mounted = true;
        }
      },
      p(new_ctx, dirty) {
        ctx = new_ctx;
        if (
          /*msg*/
          ctx[18].is_notice
        ) {
          if (if_block0) {
            if_block0.p(ctx, dirty);
          } else {
            if_block0 = create_if_block_20(ctx);
            if_block0.c();
            if_block0.m(div9, t0);
          }
        } else if (if_block0) {
          if_block0.d(1);
          if_block0 = null;
        }
        if (dirty[0] & /*messages*/
        1) show_if_6 = callBool(
          "isNewMessageDay",
          /*messages*/
          ctx[0],
          /*idx*/
          ctx[20]
        );
        if (show_if_6) {
          if (if_block1) {
            if_block1.p(ctx, dirty);
          } else {
            if_block1 = create_if_block_19(ctx);
            if_block1.c();
            if_block1.m(div9, t1);
          }
        } else if (if_block1) {
          if_block1.d(1);
          if_block1 = null;
        }
        if (dirty[0] & /*messages*/
        1) {
          set_style(div0, "display", showAvatar(
            /*msg*/
            ctx[18]
          ) ? "" : "none");
        }
        if (dirty[0] & /*messages*/
        1 && t5_value !== (t5_value = callStr(
          "messageTitleLabel",
          /*msg*/
          ctx[18]
        ) + "")) set_data(t5, t5_value);
        if (dirty[0] & /*messages*/
        1 && div1_class_value !== (div1_class_value = "message-agent-name " + callStr(
          "messageTitleClass",
          /*msg*/
          ctx[18]
        ) + /*msg*/
        (ctx[18].terminal ? " terminal-actor-label" : ""))) {
          attr(div1, "class", div1_class_value);
        }
        if (dirty[0] & /*messages*/
        1) {
          set_style(div1, "display", callBool(
            "showMessageTitle",
            /*msg*/
            ctx[18],
            /*idx*/
            ctx[20],
            /*messages*/
            ctx[0]
          ) ? "" : "none");
        }
        if (dirty[0] & /*messages*/
        1 && t11_value !== (t11_value = callStr(
          "thinkingBubbleLineText",
          /*msg*/
          ctx[18]
        ) + "")) set_data(t11, t11_value);
        if (dirty[0] & /*messages*/
        1 && em_data_shimmer_text_value !== (em_data_shimmer_text_value = callStr(
          "thinkingBubbleLineText",
          /*msg*/
          ctx[18]
        ))) {
          attr(em, "data-shimmer-text", em_data_shimmer_text_value);
        }
        if (dirty[0] & /*messages*/
        1) {
          set_style(
            div4,
            "display",
            /*msg*/
            ctx[18].thinking ? "" : "none"
          );
        }
        if (dirty[0] & /*messages*/
        1) show_if_5 = /*msg*/
        ctx[18].terminal && callBool(
          "terminalMessageCollapsed",
          /*msg*/
          ctx[18],
          /*idx*/
          ctx[20],
          /*messages*/
          ctx[0]
        );
        if (show_if_5) {
          if (if_block2) {
            if_block2.p(ctx, dirty);
          } else {
            if_block2 = create_if_block_18(ctx);
            if_block2.c();
            if_block2.m(div8, t14);
          }
        } else if (if_block2) {
          if_block2.d(1);
          if_block2 = null;
        }
        if (current_block_type === (current_block_type = select_block_type(ctx, dirty)) && if_block3) {
          if_block3.p(ctx, dirty);
        } else {
          if_block3.d(1);
          if_block3 = current_block_type(ctx);
          if (if_block3) {
            if_block3.c();
            if_block3.m(div5, null);
          }
        }
        if (dirty[0] & /*messages*/
        1 && div5_class_value !== (div5_class_value = "message-bubble" + bubbleClass(
          /*msg*/
          ctx[18]
        ))) {
          attr(div5, "class", div5_class_value);
        }
        if (dirty[0] & /*messages*/
        1) {
          set_style(
            div5,
            "display",
            /*bubbleVisible*/
            ctx[8](
              /*msg*/
              ctx[18],
              /*idx*/
              ctx[20]
            ) ? "" : "none"
          );
        }
        if (dirty[0] & /*messages*/
        1) {
          each_value_4 = ensure_array_like(callArr(
            "messageSourceChips",
            /*msg*/
            ctx[18]
          ));
          each_blocks_1 = update_keyed_each(each_blocks_1, dirty, get_key, 1, ctx, each_value_4, each0_lookup, div6, destroy_block, create_each_block_4, null, get_each_context_4);
        }
        if (dirty[0] & /*messages*/
        1) {
          set_style(div6, "display", callBool(
            "shouldRenderMessageContent",
            /*msg*/
            ctx[18],
            /*idx*/
            ctx[20],
            /*messages*/
            ctx[0]
          ) && callBool(
            "messageHasSourceChips",
            /*msg*/
            ctx[18]
          ) ? "" : "none");
        }
        if (dirty[0] & /*messages*/
        1 && t17_value !== (t17_value = callObj(
          "messageToolTraceSummary",
          /*msg*/
          ctx[18]
        ).label + "")) set_data(t17, t17_value);
        if (dirty[0] & /*messages*/
        1 && t19_value !== (t19_value = callObj(
          "messageToolTraceSummary",
          /*msg*/
          ctx[18]
        ).detail + "")) set_data(t19, t19_value);
        if (dirty[0] & /*messages*/
        1) {
          set_style(div7, "display", callBool(
            "shouldRenderMessageContent",
            /*msg*/
            ctx[18],
            /*idx*/
            ctx[20],
            /*messages*/
            ctx[0]
          ) && callObj(
            "messageToolTraceSummary",
            /*msg*/
            ctx[18]
          ).visible ? "" : "none");
        }
        if (dirty[0] & /*messages*/
        1) show_if_3 = callBool(
          "shouldRenderMessageContent",
          /*msg*/
          ctx[18],
          /*idx*/
          ctx[20],
          /*messages*/
          ctx[0]
        ) && call(
          "messageProgress",
          /*msg*/
          ctx[18]
        );
        if (show_if_3) {
          if (if_block4) {
            if_block4.p(ctx, dirty);
          } else {
            if_block4 = create_if_block_16(ctx);
            if_block4.c();
            if_block4.m(div8, t21);
          }
        } else if (if_block4) {
          if_block4.d(1);
          if_block4 = null;
        }
        if (dirty[0] & /*messages*/
        1) show_if_2 = callBool(
          "shouldRenderMessageContent",
          /*msg*/
          ctx[18],
          /*idx*/
          ctx[20],
          /*messages*/
          ctx[0]
        ) && /*msg*/
        ctx[18].file_output && /*msg*/
        ctx[18].file_output.path;
        if (show_if_2) {
          if (if_block5) {
            if_block5.p(ctx, dirty);
          } else {
            if_block5 = create_if_block_15(ctx);
            if_block5.c();
            if_block5.m(div8, t22);
          }
        } else if (if_block5) {
          if_block5.d(1);
          if_block5 = null;
        }
        if (dirty[0] & /*messages*/
        1) show_if_1 = callBool(
          "shouldRenderMessageContent",
          /*msg*/
          ctx[18],
          /*idx*/
          ctx[20],
          /*messages*/
          ctx[0]
        ) && /*msg*/
        ctx[18].folder_output && /*msg*/
        ctx[18].folder_output.path;
        if (show_if_1) {
          if (if_block6) {
            if_block6.p(ctx, dirty);
          } else {
            if_block6 = create_if_block_13(ctx);
            if_block6.c();
            if_block6.m(div8, t23);
          }
        } else if (if_block6) {
          if_block6.d(1);
          if_block6 = null;
        }
        if (dirty[0] & /*messages*/
        1) show_if = callBool(
          "shouldRenderMessageContent",
          /*msg*/
          ctx[18],
          /*idx*/
          ctx[20],
          /*messages*/
          ctx[0]
        ) && /*msg*/
        ctx[18].images && /*msg*/
        ctx[18].images.length;
        if (show_if) {
          if (if_block7) {
            if_block7.p(ctx, dirty);
          } else {
            if_block7 = create_if_block_12(ctx);
            if_block7.c();
            if_block7.m(div8, t24);
          }
        } else if (if_block7) {
          if_block7.d(1);
          if_block7 = null;
        }
        if (dirty[0] & /*messages, toggleTool*/
        17) {
          each_value_1 = ensure_array_like(callBool(
            "shouldRenderMessageContent",
            /*msg*/
            ctx[18],
            /*idx*/
            ctx[20],
            /*messages*/
            ctx[0]
          ) ? (
            /*msg*/
            ctx[18].tools || []
          ) : []);
          each_blocks = update_keyed_each(each_blocks, dirty, get_key_1, 1, ctx, each_value_1, each1_lookup, infring_tool_card_stack_shell, destroy_block, create_each_block_1, null, get_each_context_1);
        }
        if (dirty[0] & /*messages*/
        1 && infring_message_meta_shell_state_value !== (infring_message_meta_shell_state_value = callStr(
          "messageMetadataShellState",
          /*msg*/
          ctx[18],
          /*idx*/
          ctx[20],
          /*messages*/
          ctx[0]
        ))) {
          set_custom_element_data(infring_message_meta_shell, "state", infring_message_meta_shell_state_value);
        }
        if (dirty[0] & /*messages*/
        1 && infring_chat_stream_shell_class_value !== (infring_chat_stream_shell_class_value = "message " + /*msgClass*/
        ctx[7](
          /*msg*/
          ctx[18],
          /*idx*/
          ctx[20]
        ))) {
          set_custom_element_data(infring_chat_stream_shell, "class", infring_chat_stream_shell_class_value);
        }
        if (dirty[0] & /*messages*/
        1 && infring_chat_stream_shell_data_message_dom_id_value !== (infring_chat_stream_shell_data_message_dom_id_value = callStr(
          "messageDomId",
          /*msg*/
          ctx[18],
          /*idx*/
          ctx[20]
        ))) {
          set_custom_element_data(infring_chat_stream_shell, "data-message-dom-id", infring_chat_stream_shell_data_message_dom_id_value);
        }
        if (dirty[0] & /*messages*/
        1 && infring_chat_stream_shell_data_origin_kind_value !== (infring_chat_stream_shell_data_origin_kind_value = callStr(
          "messageOriginKind",
          /*msg*/
          ctx[18]
        ))) {
          set_custom_element_data(infring_chat_stream_shell, "data-origin-kind", infring_chat_stream_shell_data_origin_kind_value);
        }
        if (dirty[0] & /*messages*/
        1 && infring_chat_stream_shell_role_value !== (infring_chat_stream_shell_role_value = /*msg*/
        ctx[18].role || "")) {
          set_custom_element_data(infring_chat_stream_shell, "role", infring_chat_stream_shell_role_value);
        }
        if (dirty[0] & /*messages*/
        1 && infring_chat_stream_shell_grouped_value !== (infring_chat_stream_shell_grouped_value = callBool(
          "isGrouped",
          /*idx*/
          ctx[20],
          /*messages*/
          ctx[0]
        ) ? "true" : null)) {
          set_custom_element_data(infring_chat_stream_shell, "grouped", infring_chat_stream_shell_grouped_value);
        }
        if (dirty[0] & /*messages*/
        1 && infring_chat_stream_shell_streaming_value !== (infring_chat_stream_shell_streaming_value = /*msg*/
        ctx[18].streaming ? "true" : null)) {
          set_custom_element_data(infring_chat_stream_shell, "streaming", infring_chat_stream_shell_streaming_value);
        }
        if (dirty[0] & /*messages*/
        1 && infring_chat_stream_shell_thinking_value !== (infring_chat_stream_shell_thinking_value = /*msg*/
        ctx[18].thinking ? "true" : null)) {
          set_custom_element_data(infring_chat_stream_shell, "thinking", infring_chat_stream_shell_thinking_value);
        }
        if (dirty[0] & /*hoveredIdx, messages*/
        3 && infring_chat_stream_shell_hovered_value !== (infring_chat_stream_shell_hovered_value = /*hoveredIdx*/
        ctx[1] === /*idx*/
        ctx[20] ? "true" : null)) {
          set_custom_element_data(infring_chat_stream_shell, "hovered", infring_chat_stream_shell_hovered_value);
        }
        if (dirty[0] & /*messages*/
        1) {
          set_style(
            infring_chat_stream_shell,
            "display",
            /*msg*/
            ctx[18].is_notice ? "none" : ""
          );
        }
        if (dirty[0] & /*messages*/
        1 && div9_id_value !== (div9_id_value = callStr(
          "messageDomId",
          /*msg*/
          ctx[18],
          /*idx*/
          ctx[20]
        ))) {
          attr(div9, "id", div9_id_value);
        }
        if (dirty[0] & /*messages*/
        1 && div9_data_msg_idx_value !== (div9_data_msg_idx_value = /*idx*/
        ctx[20])) {
          attr(div9, "data-msg-idx", div9_data_msg_idx_value);
        }
      },
      d(detaching) {
        if (detaching) {
          detach(div9);
        }
        if (if_block0) if_block0.d();
        if (if_block1) if_block1.d();
        if (if_block2) if_block2.d();
        if_block3.d();
        for (let i = 0; i < each_blocks_1.length; i += 1) {
          each_blocks_1[i].d();
        }
        if (if_block4) if_block4.d();
        if (if_block5) if_block5.d();
        if (if_block6) if_block6.d();
        if (if_block7) if_block7.d();
        for (let i = 0; i < each_blocks.length; i += 1) {
          each_blocks[i].d();
        }
        mounted = false;
        run_all(dispose);
      }
    };
  }
  function create_if_block(ctx) {
    let div;
    let button;
    let mounted;
    let dispose;
    return {
      c() {
        div = element("div");
        button = element("button");
        button.textContent = "Expand";
        attr(button, "class", "btn btn-ghost btn-sm");
        attr(button, "type", "button");
        set_style(div, "display", "flex");
        set_style(div, "justify-content", "center");
        set_style(div, "padding", "10px 0 2px");
      },
      m(target, anchor) {
        insert(target, div, anchor);
        append(div, button);
        if (!mounted) {
          dispose = listen(
            button,
            "click",
            /*click_handler_3*/
            ctx[16]
          );
          mounted = true;
        }
      },
      p: noop,
      d(detaching) {
        if (detaching) {
          detach(div);
        }
        mounted = false;
        dispose();
      }
    };
  }
  function create_fragment(ctx) {
    let div;
    let each_blocks = [];
    let each_1_lookup = /* @__PURE__ */ new Map();
    let t;
    let show_if = canExpand();
    let each_value = ensure_array_like(
      /*messages*/
      ctx[0]
    );
    const get_key = (ctx2) => renderKey(
      /*msg*/
      ctx2[18],
      /*idx*/
      ctx2[20]
    );
    for (let i = 0; i < each_value.length; i += 1) {
      let child_ctx = get_each_context(ctx, each_value, i);
      let key = get_key(child_ctx);
      each_1_lookup.set(key, each_blocks[i] = create_each_block(key, child_ctx));
    }
    let if_block = show_if && create_if_block(ctx);
    return {
      c() {
        div = element("div");
        for (let i = 0; i < each_blocks.length; i += 1) {
          each_blocks[i].c();
        }
        t = space();
        if (if_block) if_block.c();
        attr(div, "class", "chat-thread");
      },
      m(target, anchor) {
        insert(target, div, anchor);
        for (let i = 0; i < each_blocks.length; i += 1) {
          if (each_blocks[i]) {
            each_blocks[i].m(div, null);
          }
        }
        append(div, t);
        if (if_block) if_block.m(div, null);
      },
      p(ctx2, dirty) {
        if (dirty[0] & /*messages, msgClass, hoveredIdx, onMouseEnter, onMouseLeave, onMetaAction, toggleTool, bubbleVisible, expandTerminal*/
        511) {
          each_value = ensure_array_like(
            /*messages*/
            ctx2[0]
          );
          each_blocks = update_keyed_each(each_blocks, dirty, get_key, 1, ctx2, each_value, each_1_lookup, div, destroy_block, create_each_block, t, get_each_context);
        }
        if (show_if) if_block.p(ctx2, dirty);
      },
      i: noop,
      o: noop,
      d(detaching) {
        if (detaching) {
          detach(div);
        }
        for (let i = 0; i < each_blocks.length; i += 1) {
          each_blocks[i].d();
        }
        if (if_block) if_block.d();
      }
    };
  }
  function cp() {
    return typeof window !== "undefined" && window.InfringChatPage || null;
  }
  function call(fn) {
    var p = cp();
    if (!p || typeof p[fn] !== "function") return void 0;
    var args = Array.prototype.slice.call(arguments, 1);
    return p[fn].apply(p, args);
  }
  function callBool(fn) {
    var result = call.apply(null, arguments);
    return !!result;
  }
  function callStr(fn) {
    var result = call.apply(null, arguments);
    return result == null ? "" : String(result);
  }
  function callArr(fn) {
    var result = call.apply(null, arguments);
    return Array.isArray(result) ? result : [];
  }
  function callObj(fn) {
    var result = call.apply(null, arguments);
    return result && typeof result === "object" ? result : {};
  }
  function triggerNotice(msg) {
    var p = cp();
    if (p && typeof p.triggerNoticeAction === "function") p.triggerNoticeAction(msg);
  }
  function expandDisplayed() {
    var p = cp();
    if (p && typeof p.expandDisplayedMessages === "function") p.expandDisplayedMessages();
  }
  function bubbleClass(msg) {
    var p = cp();
    var role = String(msg && msg.role || "").toLowerCase();
    var isAgent = role === "agent" || role === "assistant" || role === "system";
    var r = "";
    if (isAgent && !msg.thinking && !msg.isHtml) r += " markdown-body";
    if (!msg.thoughtStreaming && callBool("isErrorMessage", msg)) r += " message-error";
    if (msg.thoughtStreaming) r += " thinking-live";
    if (msg._finish_bounce) r += " message-finish-bounce";
    return r;
  }
  function showAvatar(msg) {
    var role = String(msg && msg.role || "").toLowerCase();
    if (role !== "agent") return false;
    var p = cp();
    return !(p && typeof p.isCurrentAgentArchived === "function" && p.isCurrentAgentArchived());
  }
  function canExpand() {
    var p = cp();
    return !!(p && p.canExpandDisplayedMessages);
  }
  function renderKey(msg, idx) {
    return callStr("messageRenderKey", msg, idx) || String(idx);
  }
  function instance($$self, $$props, $$invalidate) {
    let messages = [];
    let hoveredIdx = -1;
    let unsub;
    onMount(function() {
      var s = typeof window !== "undefined" && window.InfringChatStore;
      if (s && s.filteredMessages) {
        unsub = s.filteredMessages.subscribe(function(val) {
          $$invalidate(0, messages = Array.isArray(val) ? val : []);
        });
      }
    });
    onDestroy(function() {
      if (typeof unsub === "function") unsub();
    });
    function onMouseEnter(msg, idx) {
      $$invalidate(1, hoveredIdx = idx);
      var p = cp();
      if (p && typeof p.setHoveredMessage === "function") p.setHoveredMessage(msg, idx);
    }
    function onMouseLeave() {
      $$invalidate(1, hoveredIdx = -1);
      var p = cp();
      if (p && typeof p.clearHoveredMessage === "function") p.clearHoveredMessage();
    }
    function toggleTool(tool) {
      tool.expanded = !tool.expanded;
      $$invalidate(0, messages);
    }
    function onMetaAction(e, msg, idx) {
      var p = cp();
      if (p && typeof p.handleMessageMetaAction === "function") {
        p.handleMessageMetaAction(e, msg, idx, messages);
      }
    }
    function expandTerminal(msg, idx) {
      var p = cp();
      if (p && typeof p.expandTerminalMessage === "function") p.expandTerminalMessage(msg, idx, messages);
    }
    function msgClass(msg, idx) {
      var r = callStr("messageRoleClass", msg);
      r += msg.thinking ? " thinking" : "";
      r += msg.streaming ? " streaming" : "";
      r += callBool("isGrouped", idx, messages) ? " grouped" : "";
      r += callBool("showMessageTail", msg, idx, messages) ? " has-tail" : "";
      r += !callBool("isLastInSourceRun", idx, messages) ? " has-next-in-run" : "";
      r += hoveredIdx === idx ? " hover-linked" : "";
      r += callBool("isMessageMetaCollapsed", msg, idx, messages) ? " meta-collapsed" : " meta-expanded";
      r += callBool("isMessageMetaReserveSpace", msg, idx, messages) ? " meta-reserved" : "";
      return r;
    }
    function bubbleVisible(msg, idx) {
      if (msg.thinking) return false;
      if (msg.terminal && callBool("terminalMessageCollapsed", msg, idx, messages)) return false;
      var p = cp();
      if (!p) return false;
      return !!(msg.text && msg.text.trim() || callBool("messageHasTools", msg) || callBool("messageHasSourceChips", msg) || callObj("messageToolTraceSummary", msg).visible || call("messageProgress", msg) || msg.file_output && msg.file_output.path || msg.folder_output && msg.folder_output.path || msg.images && msg.images.length);
    }
    const click_handler = (msg) => triggerNotice(msg);
    const click_handler_1 = (msg, idx) => expandTerminal(msg, idx);
    const keydown_handler = (msg, idx, e) => {
      if (e.key === "Enter" || e.key === " ") {
        e.preventDefault();
        expandTerminal(msg, idx);
      }
    };
    const click_handler_2 = (tool) => toggleTool(tool);
    const message_meta_action_handler = (msg, idx, e) => onMetaAction(e, msg, idx);
    const mouseenter_handler = (msg, idx) => onMouseEnter(msg, idx);
    const mouseleave_handler = () => onMouseLeave();
    const click_handler_3 = () => expandDisplayed();
    return [
      messages,
      hoveredIdx,
      onMouseEnter,
      onMouseLeave,
      toggleTool,
      onMetaAction,
      expandTerminal,
      msgClass,
      bubbleVisible,
      click_handler,
      click_handler_1,
      keydown_handler,
      click_handler_2,
      message_meta_action_handler,
      mouseenter_handler,
      mouseleave_handler,
      click_handler_3
    ];
  }
  var Chat_thread_shell = class extends SvelteComponent {
    constructor(options) {
      super();
      init(this, options, instance, create_fragment, safe_not_equal, {}, null, [-1, -1]);
    }
  };
  customElements.define("infring-chat-thread-shell", create_custom_element(Chat_thread_shell, {}, [], [], false));
  var chat_thread_shell_svelte_default = Chat_thread_shell;
})();
