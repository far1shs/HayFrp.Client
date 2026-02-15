<template>
  <div class="flex flex-col gap-3 relative h-[calc(100vh-108px)] tunnel-container overflow-y-auto">
    <div v-if="accountsData && accountsData.length === 0"
      class="flex flex-col gap-5 items-center justify-center h-full">
      <UsersRoundIcon class="size-12" />
      请到设置里面添加一下账号
    </div>

    <div v-if="isInitializing" class="flex items-center justify-center h-full">
      <Loading />
    </div>

    <div v-else v-for="(account, accIndex) in accountsData" :key="accIndex" class="flex flex-col gap-3">
      <div class="flex items-center justify-between py-1">
        <span class="text-xs border bg-background rounded-sm shadow-sm px-1">{{ account.name }}</span>
        <div>
          <ButtonGroup>
            <Button @click.stop="handleOpenWebView(account)" size="icon" variant="outline"
              class="size-5 hover:text-blue-500">
              <GlobeIcon class="size-2.5" />
            </Button>
            <Button @click.stop="refreshAccount(accIndex)" variant="outline" size="icon" class="size-5 hover:bg-accent">
              <RotateCwIcon class="size-3" />
            </Button>
          </ButtonGroup>
        </div>
      </div>

      <div v-if="account.loading" class="flex items-center justify-center h-32">
        <Loading class="scale-75" />
      </div>

      <div v-else-if="account.error"
        class="flex items-center justify-center h-20 border border-dashed rounded-md bg-destructive/5">
        <div class="text-sm text-destructive flex flex-col items-center gap-1">
          <span>获取数据失败</span>
          <span class="text-xs opacity-75">{{ account.errorMessage }}</span>
        </div>
      </div>

      <div v-else class="grid grid-cols-2 md:grid-cols-3 lg:grid-cols-4 gap-3">
        <Item v-for="(item, index) in account.tunnels" :key="index" variant="outline"
          @click="handleItemClick(item, $event)" :class="[
            'transition-all duration-300 bg-background dark:bg-[#151515] hover:shadow-md cursor-pointer',
            item.status !== 'true' ? 'opacity-70 cursor-not-allowed' : ''
          ]">
          <ItemContent>
            <div class="flex flex-col gap-1">
              <div class="flex items-center justify-between">
                <div class="text-sm min-w-0">
                  <span class="truncate">{{ item.proxy_name }} <span class="text-muted-foreground">#{{ item.id
                  }}</span></span>
                </div>
                <Switch v-if="item.status === 'true'" v-model:model-value="useProcessIsRunning(item.id).value"
                  @click.stop @update:model-value="(value) => handleTunnelSwitch(item.id, value, account.csrf)" />
                <span v-else class="text-xs text-muted-foreground">控制台解锁</span>
              </div>

              <Separator class="my-1.5" />

              <div class="flex items-center">
                <span class="text-sm text-muted-foreground">{{ item.local_ip }}:{{ item.local_port }} -> {{
                  item.remote_port }}</span>
              </div>

              <div class="mt-1 flex items-center justify-between">
                <ButtonGroup class="h-5.5">
                  <Badge variant="outline" class="rounded-sm text-[10px] px-1.5"><span>{{ item.proxy_type.toUpperCase()
                  }}</span></Badge>
                  <Badge v-if="item.use_encryption == 'true'" variant="outline" class="rounded-sm text-[10px] px-1.5">加密
                  </Badge>
                  <Badge v-if="item.use_compression == 'true'" variant="outline" class="rounded-sm text-[10px] px-1.5">
                    压缩</Badge>
                </ButtonGroup>

                <ButtonGroup>
                  <Button @click.stop="handleDeleteTunnel(item.id, account.csrf)" size="icon" variant="outline"
                    class="size-5.5 rounded-sm hover:text-destructive">
                    <Trash2Icon class="size-2.5" />
                  </Button>
                  <Button @click.stop="handleCopy(item.id, item.node_domain, item.remote_port)" size="icon"
                    variant="outline" class="size-5.5 rounded-sm hover:text-primary">
                    <CopyIcon class="size-2.5" />
                  </Button>
                </ButtonGroup>
              </div>
            </div>
          </ItemContent>
        </Item>

        <div v-if="account.tunnels.length === 0" class="col-span-full py-4 text-center text-xs text-muted-foreground">
          该账号下暂无隧道
        </div>
      </div>
    </div>

    <TunnelDetail v-if="selectedTunnelId" :tunnel="activeTunnel" :initialRect="expandRect"
      @close="selectedTunnelId = null" @switch="(value) => handleDetailSwitch(value)" />
  </div>
</template>

<script setup lang="ts">
import { Trash2Icon, CopyIcon, RotateCwIcon, UsersRoundIcon, GlobeIcon } from "lucide-vue-next";
import Loading from "@/components/Loading.vue";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Item, ItemContent } from "@/components/ui/item";
import { Separator } from "@/components/ui/separator";
import { Switch } from "@/components/ui/switch";
import { API_URL } from "@/lib/env";
import { computed, onMounted, onUnmounted, ref } from "vue";
import { toast } from "vue-sonner";
import { ButtonGroup } from "@/components/ui/button-group";
import { useFrpStore } from "@/store/frp.ts";
import TunnelDetail from "@/components/TunnelDetail.vue";
import { writeText } from "@tauri-apps/plugin-clipboard-manager";
import { useNavStore } from "@/store/nav.ts";
import { load } from "@tauri-apps/plugin-store";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-shell";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

interface AccountConfig {
  name: string;
  csrf: string;
}

interface AccountData extends AccountConfig {
  tunnels: any[];
  loading: boolean;
  error: boolean;
  errorMessage: string;
}

const frpStore = useFrpStore();
const navStore = useNavStore();

const isInitializing = ref(true);
const accountsData = ref<AccountData[]>([]);

const selectedTunnelId = ref<string | null>(null);
const expandRect = ref({ top: 0, left: 0, width: 0, height: 0 });

let unlistenAccountsUpdated: UnlistenFn | null = null;

const handleRefreshAll = async () => {
  const store = await load("settings.json", {
    defaults: {
      accounts: [],
    },
    autoSave: true,
  });

  const savedAccounts = (await store.get<AccountConfig[]>("accounts")) || [];

  if (savedAccounts.length === 0) {
    isInitializing.value = false;
    accountsData.value = [];
    toast.info("暂无账号配置");
    return;
  }

  if (accountsData.value.length !== savedAccounts.length) {
    accountsData.value = savedAccounts.map(acc => ({
      ...acc,
      tunnels: [],
      loading: true,
      error: false,
      errorMessage: ""
    }));
  } else {
    accountsData.value.forEach(acc => acc.loading = true);
  }

  isInitializing.value = false;

  const fetchPromises = accountsData.value.map(async (account, index) => {
    try {
      const csrf: string = await invoke("secure_decrypt", {
        encryptedBase64: account.csrf
      });

      const req = await fetch(`${API_URL.value}/proxy`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          type: "list",
          csrf: csrf
        })
      });

      if (!req.ok) throw new Error(`${req.status} ${req.statusText}`);

      const json = await req.json();
      if (json.status !== 200) throw new Error(json.message);

      accountsData.value[index].tunnels = json.proxies || [];
      accountsData.value[index].error = false;
      return { status: "fulfilled", name: account.name };

    } catch (e: any) {
      accountsData.value[index].error = true;
      accountsData.value[index].errorMessage = e.message || "未知错误";
      return Promise.reject({ name: account.name, message: e.message });
    } finally {
      accountsData.value[index].loading = false;
    }
  });

  const results = await Promise.allSettled(fetchPromises);

  const failCount = results.filter(r => r.status === "rejected").length;
  if (failCount > 0) {
    toast.warning(`刷新失败 ${failCount} 个`);
  }
};

const refreshAccount = async (index: number) => {
  const account = accountsData.value[index];
  if (!account) return;

  account.loading = true;
  account.error = false;

  try {
    const csrf: string = await invoke("secure_decrypt", {
      encryptedBase64: account.csrf
    });

    const req = await fetch(`${API_URL.value}/proxy`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        type: "list",
        csrf: csrf
      })
    });

    if (!req.ok) throw new Error(`${req.status} ${req.statusText}`);

    const json = await req.json();
    if (json.status !== 200) throw new Error(json.message);

    account.tunnels = json.proxies || [];

    toast.success(`已刷新账号 ${account.name}`);

  } catch (e: any) {
    account.error = true;
    account.errorMessage = e.message || "未知错误";
    toast.error(`刷新失败: ${account.name}`, { description: e.message });
  } finally {
    account.loading = false;
  }
};

const handleTunnelSwitch = async (id: string, value: boolean, csrf: string) => {
  if (value) {
    try {
      const reqPromise = (async () => {
        const token: string = await invoke("secure_decrypt", {
          encryptedBase64: csrf
        });

        const res = await fetch(`${API_URL.value}/esirun`, {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ id, csrf: token, type: "create" }),
        });

        if (!res.ok) throw new Error(`HTTP ${res.status}`);
        const json = await res.json();
        if (json.status !== 200) throw new Error(json.message);

        await frpStore.run(
          id,
          ["-esirun", json.csrf, "--disable-log-color"]
        );
      })();

      toast.promise(reqPromise, {
        loading: `正在获取配置 #${id}`,
        success: `启动成功 #${id}`,
        error: (err: any) => `启动失败 #${id}, ${err.message}`,
      });

    } catch (err) {
      toast.error(`启动失败 #${id}`, { description: err?.toString() });
    }
  } else {
    await frpStore.kill(id);
    toast.success(`关闭成功 #${id}`);
  }
};

const handleDetailSwitch = async (value: { id: string, value: boolean }) => {
  let targetCsrf = "";
  for (const acc of accountsData.value) {
    if (acc.tunnels.some(t => t.id === value.id)) {
      targetCsrf = acc.csrf;
      break;
    }
  }

  if (targetCsrf) {
    const csrf: string = await invoke("secure_decrypt", {
      encryptedBase64: targetCsrf
    });

    handleTunnelSwitch(value.id, value.value, csrf);
  } else {
    toast.error("未找到对应账号信息");
  }
};

const handleCopy = async (id: string, domain: string, port: string) => {
  await writeText(`${domain}:${port}`);
  toast.success(`复制成功 #${id}`, {
    description: `${domain}:${port}`
  });
};

const useProcessIsRunning = (id: string) => {
  return computed({
    get: () => frpStore.processes[id]?.isRunning || false,
    set: () => { }
  });
}

const handleItemClick = (item: any, event: MouseEvent) => {
  const rect = (event.currentTarget as HTMLElement).getBoundingClientRect();
  expandRect.value = {
    top: rect.top,
    left: rect.left,
    width: rect.width,
    height: rect.height
  };
  selectedTunnelId.value = item.id;
};

const handleOpenWebView = async (account: AccountData) => {
  try {
    // 解密 csrf
    const decryptedCsrf: string = await invoke("secure_decrypt", {
      encryptedBase64: account.csrf
    });
    
    // 在 URL 中传入 csrf，浏览器插件会读取并注入
    const url = `https://console.hayfrp.com/proxies#csrf=${encodeURIComponent(decryptedCsrf)}`;
    await open(url);
    toast.success(`已在浏览器中打开控制台`);
  } catch (err: any) {
    console.error("打开浏览器失败:", err);
    toast.error("打开浏览器失败", {description: err?.message || err?.toString()});
  }
};

const handleDeleteTunnel = async (id: string, csrf: string) => {
  const reqPromise = (async () => {
    const token: string = await invoke("secure_decrypt", {
      encryptedBase64: csrf
    });

    const res = await fetch(`${API_URL.value}/proxy`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ id, csrf: token, type: "remove" }),
    });

    if (!res.ok) throw new Error(`HTTP ${res.status}`);
    const json = await res.json();
    if (json.status !== 200) throw new Error(json.message);

    accountsData.value.forEach(acc => {
      acc.tunnels = acc.tunnels.filter(t => t.id !== id);
    });
  })();

  toast.promise(reqPromise, {
    loading: `正在删除 #${id}`,
    success: `删除成功 #${id}`,
    error: (err: any) => `删除 失败 #${id}, ${err.message}`,
  });
}

const activeTunnel = computed(() => {
  if (!selectedTunnelId.value) return null;
  for (const acc of accountsData.value) {
    const found = acc.tunnels.find(t => t.id === selectedTunnelId.value);
    if (found) return found;
  }
  return null;
});

onMounted(async () => {
  await handleRefreshAll();
  navStore.setRefreshAction(handleRefreshAll);
  
  // 监听账号更新事件
  unlistenAccountsUpdated = await listen("accounts-updated", async () => {
    // 重新刷新所有账号数据
    await handleRefreshAll();
    toast.success("检测到新账号，已自动刷新");
  });
});

onUnmounted(async () => {
  navStore.setRefreshAction(null);
  if (unlistenAccountsUpdated) {
    unlistenAccountsUpdated();
  }
});
</script>