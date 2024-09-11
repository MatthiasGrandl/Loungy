// deno-lint-ignore-file no-explicit-any
import React from "https://esm.sh/react@18.2.0";
import Reconciler, { HostConfig } from "https://esm.sh/react-reconciler@0.29.0";

interface TreeNode {
  id: number;
  type: Type;
  props: { [key: string]: any };
  children: TreeNode[];
  text?: string;
  hostContext: HostContext;
}

type Type = string;
type Props = { [key: string]: any };
type Container = {
  rootNode: TreeNode | null;
  pendingChildren: TreeNode[];
};
type HostContext = {
  id: number;
}
type Instance = TreeNode;
type TextInstance = TreeNode;
type SuspenseInstance = never;
type HydratableInstance = never;
type PublicInstance = TreeNode;
type UpdatePayload = Props;
type ChildSet = TreeNode[];
type TimeoutHandle = ReturnType<typeof setTimeout>;
type NoTimeout = -1;

const createComponent = (type: Type, props: Props, children: TreeNode[], hostContext: HostContext, text?: string): TreeNode => {
  hostContext.id += 1;
  return {
    id: hostContext.id,
    type,
    props,
    children,
    hostContext,
    text
  };
};

// Create the host config
const hostConfig: HostConfig<
  Type,
  Props,
  Container,
  Instance,
  TextInstance,
  SuspenseInstance,
  HydratableInstance,
  PublicInstance,
  HostContext,
  UpdatePayload,
  ChildSet,
  TimeoutHandle,
  NoTimeout
> = {
  supportsPersistence: true,

  createInstance(
    type: Type,
    props: Props,
    _rootContainerInstance: Container,
    hostContext: HostContext,
    _internalInstanceHandle: Reconciler.OpaqueHandle
  ): Instance {
    return createComponent(type, props, [], hostContext);
  },

  createTextInstance(
    text: string,
    _rootContainerInstance: Container,
    hostContext: HostContext,
    _internalInstanceHandle: Reconciler.OpaqueHandle
  ): TextInstance {
    return createComponent('loungy:text', {}, [], hostContext, text);
  },

  appendInitialChild(
    parentInstance: Instance,
    child: Instance | TextInstance
  ): void {
    parentInstance.children.push(child);
  },

  finalizeInitialChildren(
    _instance: Instance,
    _type: Type,
    _props: Props,
    _rootContainerInstance: Container,
    _hostContext: HostContext
  ): boolean {
    return false;
  },

  prepareUpdate(
    _instance: Instance,
    _type: Type,
    oldProps: Props,
    newProps: Props,
    _rootContainerInstance: Container,
    _hostContext: HostContext
  ): UpdatePayload | null {
    const updatePayload: UpdatePayload = {};
    let needsUpdate = false;

    for (const key in newProps) {
      if (oldProps[key] !== newProps[key]) {
        updatePayload[key] = newProps[key];
        needsUpdate = true;
      }
    }

    return needsUpdate ? updatePayload : null;
  },

  shouldSetTextContent(_type: Type, props: Props): boolean {
    return typeof props.children === 'string' || typeof props.children === 'number';
  },

  getRootHostContext(_rootContainerInstance: Container): HostContext {
    return { id: -1 };
  },

  getChildHostContext(
    parentHostContext: HostContext,
    _type: Type,
    _rootContainerInstance: Container
  ): HostContext {
    return parentHostContext;
  },

  getPublicInstance(instance: Instance | TextInstance): PublicInstance {
    return instance;
  },

  prepareForCommit(containerInfo: Container): null {
    containerInfo.pendingChildren = [];
    return null;
  },

  resetAfterCommit(containerInfo: Container): void {
    containerInfo.rootNode = containerInfo.pendingChildren[0] || null;
    containerInfo.pendingChildren = [];
  },

  preparePortalMount(_containerInfo: Container): void {
    // No-op for this example
  },

  scheduleTimeout(
    fn: (...args: unknown[]) => unknown,
    delay?: number | undefined
  ): TimeoutHandle {
    return setTimeout(fn, delay);
  },

  cancelTimeout(id: TimeoutHandle): void {
    clearTimeout(id);
  },

  noTimeout: -1 as const,

  isPrimaryRenderer: true,

  getCurrentEventPriority: () => 1,
  detachDeletedInstance: (_node: Instance): void => {
  },

  warnsIfNotActing: true,

  supportsMutation: false,

  supportsMicrotasks: false,

  supportsHydration: false,

  getInstanceFromNode(_node: any): Reconciler.OpaqueHandle | null {
    return null;
  },

  beforeActiveInstanceBlur(): void {
    // No-op for this example
  },

  afterActiveInstanceBlur(): void {
    // No-op for this example
  },

  prepareScopeUpdate(_scopeInstance: any, _instance: any): void {
    // No-op for this example
  },

  getInstanceFromScope(_scopeInstance: any): Reconciler.OpaqueHandle | null {
    return null;
  },

  // Persistence-specific methods
  cloneInstance(
    instance: Instance,
    updatePayload: UpdatePayload,
    type: Type,
    oldProps: Props,
    newProps: Props,
    _internalInstanceHandle: Reconciler.OpaqueHandle,
    keepChildren: boolean,
    _recyclableInstance: Instance
  ): Instance {
    instance.hostContext.id += 1;
    const clonedInstance: Instance = {
      ...instance,
      id: instance.hostContext.id,
      type: type,
      props: updatePayload !== null ? newProps : oldProps,
    };

    if (!keepChildren) {
      clonedInstance.children = [];
    }

    return clonedInstance;
  },

  createContainerChildSet(_container: Container): ChildSet {
    return [];
  },

  appendChildToContainerChildSet(childSet: ChildSet, child: Instance | TextInstance): void {
    childSet.push(child);
  },

  finalizeContainerChildren(container: Container, newChildren: ChildSet): void {
    container.pendingChildren = newChildren;
  },

  replaceContainerChildren(container: Container, newChildren: ChildSet): void {
    // This is where we send the entire tree to the renderer
    container.pendingChildren = newChildren;
    console.log('Sending tree to renderer:', JSON.stringify(container, null, 2));
    // In a real implementation, you would call your renderer here
    // renderTree(treeToRender);
  },

  cloneHiddenInstance(
    instance: Instance,
    _type: Type,
    props: Props,
    _internalInstanceHandle: Reconciler.OpaqueHandle
  ): Instance {
    return { ...instance, props: { ...props, style: { ...props.style, display: 'none' } } };
  },

  cloneHiddenTextInstance(
    instance: TextInstance,
    text: string,
    _internalInstanceHandle: Reconciler.OpaqueHandle
  ): TextInstance {
    return { ...instance, text, props: { style: { display: 'none' } } };
  },
};

export async function render(element: React.ReactElement) {
  const reconciler = Reconciler(hostConfig);
  const containerInfo = {
    rootNode: null,
    pendingChildren: [],
  };

  const root = reconciler.createContainer(containerInfo, 0, null, false, true, "", error => {
    console.error("Recoverable error occurred when rendering view", error)
  }, null);

  reconciler.updateContainer(element, root, null, () => {
    // This callback runs after the initial render
    console.log('Render complete');

  });

  const sleep = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms));
  while (true) {
    await sleep(Infinity);
  }

}

export default () => {
  (globalThis as unknown as { render: unknown }).render = render;
  (globalThis as unknown as { React: unknown }).React = React;
}
