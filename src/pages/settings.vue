<template>
  <div class="flex flex-col gap-3 h-[calc(100vh-108px)] overflow-y-auto">
    <!--账号-->
    <div class="flex items-center justify-between">
      <span class="text-xs text-muted-foreground ml-1.5">账号</span>
      <Dialog>
        <DialogTrigger>
          <Button variant="outline" size="icon" class="size-5 -m-2 ml-1.5 mr-0">
            <PlusIcon class="size-3"/>
          </Button>
        </DialogTrigger>

        <DialogContent>
          <DialogHeader>
            <DialogTitle>添加账号</DialogTitle>
          </DialogHeader>

          <span class="text-sm">获取 CSRF 可以在 <a target="_blank"
                                                    href="https://console.hayfrp.com/console#user">控制台</a> 看到有一个 "显示 TOKEN/CSRF 值" 按钮</span>
          <Input v-model:model-value="data.csrf" placeholder="账号 CSRF"/>
          <Button @click="handleAccount">添加</Button>
        </DialogContent>
      </Dialog>
    </div>
    <Item v-if="data.accounts.length === 0" variant="outline" class="bg-background dark:bg-[#151515]">
      <ItemMedia>
        <UsersRoundIcon class="size-5"/>
      </ItemMedia>
      <ItemContent>
        <ItemTitle>点击右上角添加一个账号</ItemTitle>
      </ItemContent>
    </Item>
    <div v-else class="flex flex-col gap-3">
      <Item v-for="(item, index) in data.accounts" :key="index" variant="outline"
            class="bg-background dark:bg-[#151515]">
        <ItemMedia>
          <Avatar v-if="item.avatar" class="rounded-md size-7 -m-1">
            <AvatarImage :src="item.avatar"/>
          </Avatar>
          <UsersRoundIcon v-else class="size-5"/>
        </ItemMedia>
        <ItemContent>
          <ItemTitle>{{ item.name }}</ItemTitle>
        </ItemContent>
        <ItemActions>
          <Dialog>
            <DialogTrigger>
              <Button variant="outline" size="icon" class="size-7 -m-2">
                <Edit3Icon class="size-3"/>
              </Button>
            </DialogTrigger>

            <DialogContent>
              <DialogHeader>
                <DialogTitle>修改账号</DialogTitle>
              </DialogHeader>

              <span class="text-sm">获取 CSRF 可以在 <a target="_blank"
                                                        href="https://console.hayfrp.com/console#user">控制台</a> 看到有一个 "显示 TOKEN/CSRF 值" 按钮</span>
              <Input v-model:model-value="data.csrf" placeholder="账号 CSRF"/>
              <Button @click="handleAccount">保存</Button>
            </DialogContent>
          </Dialog>
          <Button @click="handleDeleteAccount(item.name)" variant="outline" size="icon" class="size-7 -m-2 ml-1.5 mr-0">
            <TrashIcon class="size-3"/>
          </Button>
        </ItemActions>
      </Item>
    </div>

    <!--通用-->
    <span class="text-xs text-muted-foreground ml-1.5">通用</span>
    <Item variant="outline" class="bg-background dark:bg-[#151515]">
      <ItemMedia>
        <CircleChevronUpIcon class="size-5"/>
      </ItemMedia>
      <ItemContent>
        <ItemTitle>开机自启</ItemTitle>
      </ItemContent>
      <ItemActions>
        <Switch v-model:model-value="data.auto_start"
                @update:model-value="(value) => value ? autostartEnable() : autostartDisable()"/>
      </ItemActions>
    </Item>
    <Item variant="outline" class="bg-background dark:bg-[#151515]">
      <ItemMedia>
        <LayoutGridIcon class="size-5"/>
      </ItemMedia>
      <ItemContent>
        <ItemTitle>自启动隧道 (启动上次关闭时, 开启的隧道)</ItemTitle>
      </ItemContent>
      <ItemActions>
        <Switch v-model:model-value="data.auto_tunnel"/>
      </ItemActions>
    </Item>
    <Item variant="outline" class="bg-background dark:bg-[#151515]">
      <ItemMedia>
        <WebhookIcon class="size-5"/>
      </ItemMedia>
      <ItemContent>
        <ItemTitle>API</ItemTitle>
      </ItemContent>
      <ItemActions>
        <span v-if="API_URL !== ''" class="text-muted-foreground">{{ handleGetHomeName(API_URL) }}</span>

        <Dialog>
          <DialogTrigger>
            <Button variant="outline" size="icon" class="size-7 -m-2 ml-1.5 mr-0">
              <Edit3Icon class="size-3"/>
            </Button>
          </DialogTrigger>

          <DialogContent>
            <DialogHeader>
              <DialogTitle>API</DialogTitle>
            </DialogHeader>
            <Input v-model:model-value="data.api_url"/>
            <Button @click="handleEditAPI">保存</Button>
          </DialogContent>
        </Dialog>
      </ItemActions>
    </Item>
    <Item variant="outline" class="bg-background dark:bg-[#151515]">
      <ItemMedia>
        <AppWindowIcon class="size-5"/>
      </ItemMedia>
      <ItemContent>
        <ItemTitle>FRPC</ItemTitle>
      </ItemContent>
      <ItemActions>
        <span class="text-muted-foreground">{{ data.frpc_version }}</span>
        <Dialog>
          <DialogTrigger>
            <Button variant="outline" size="icon" class="size-7 -m-2 ml-1.5 mr-0">
              <Edit3Icon class="size-3"/>
            </Button>
          </DialogTrigger>

          <DialogContent>
            <DialogHeader>
              <DialogTitle>FRPC</DialogTitle>
            </DialogHeader>
            <span class="text-sm">例子 X:\xxx\hayfrpc.exe 或者 /xxx/hayfrpc 根据系统自行判断</span>
            <Input v-model:model-value="data.frpc_path"/>
            <Button @click="handleEditFRPC">保存</Button>
          </DialogContent>
        </Dialog>
      </ItemActions>
    </Item>

    <!--关于-->
    <span class="text-xs text-muted-foreground ml-1.5">关于</span>
    <Item variant="outline" class="bg-background dark:bg-[#151515]">
      <ItemMedia>
        <InfoIcon class="size-5"/>
      </ItemMedia>
      <ItemContent>
        <ItemTitle>HAYFRP 半三方客户端 - Far1sh</ItemTitle>
      </ItemContent>
      <ItemActions>1.0.4</ItemActions>
    </Item>
    <Item variant="outline" class="bg-background dark:bg-[#151515]" as-child>
      <a target="_blank" href="https://github.com/far1shs/HayFrp.Client">
        <ItemMedia>
          <GithubIcon class="size-5"/>
        </ItemMedia>
        <ItemContent>
          <ItemTitle>GitHub</ItemTitle>
        </ItemContent>
        <ItemActions>
          <SquareArrowOutUpRightIcon class="size-4"/>
        </ItemActions>
      </a>
    </Item>
  </div>
</template>

<script setup lang="ts">
import {Item, ItemMedia, ItemContent, ItemTitle, ItemActions} from "@/components/ui/item";
import {Switch} from "@/components/ui/switch";
import {
  CircleChevronUpIcon,
  PlusIcon,
  LayoutGridIcon,
  WebhookIcon,
  Edit3Icon,
  UsersRoundIcon,
  TrashIcon,
  AppWindowIcon,
  InfoIcon,
  GithubIcon,
  SquareArrowOutUpRightIcon
} from "lucide-vue-next";
import {
  enable as autostartEnable,
  isEnabled as autostartIsEnabled,
  disable as autostartDisable
} from "@tauri-apps/plugin-autostart";
import {onMounted, reactive, watch} from "vue";
import {load} from "@tauri-apps/plugin-store";
import {Input} from "@/components/ui/input";
import {Button} from "@/components/ui/button";
import {Dialog, DialogTrigger, DialogContent, DialogHeader, DialogTitle} from "@/components/ui/dialog";
import {API_URL, FRPC_PATH} from "@/lib/env.ts";
import {toast} from "vue-sonner";
import {Avatar, AvatarImage} from "@/components/ui/avatar";
import {invoke} from "@tauri-apps/api/core";

const data = reactive({
  api_url: "",
  auto_start: false,
  auto_tunnel: false,
  csrf: "",
  frpc_version: "",
  frpc_path: "",
  accounts: [] as any[]
});

const handleAccount = async () => {
  const store = await load("settings.json", {
    defaults: {
      accounts: []
    },
    autoSave: true
  });

  const reqPromise = (async () => {
    const res = await fetch(`${API_URL.value}/user`, {
      method: "POST",
      headers: {"Content-Type": "application/json"},
      body: JSON.stringify({csrf: data.csrf, type: "info"}),
    });

    if (!res.ok) throw new Error(`HTTP ${res.status}`);
    const json = await res.json();
    if (json.status !== 200) throw new Error(json.message);

    const accounts = await store.get<any[]>("accounts") || [];
    const csrf: string = await invoke("secure_encrypt", {
      token: data.csrf
    });

    if (accounts.find((account) => account.name === json.username)) {
      accounts.find((account) => account.name === json.username)!.csrf = csrf;
    } else {
      accounts.push({
        name: json.username,
        csrf: csrf,
        avatar: json.qid && json.qid != "" ? `https://q1.qlogo.cn/g?b=qq&nk=${json.qid}&s=640` : null,
        status: true
      });
    }

    await store.set("accounts", accounts);
    data.accounts = await store.get("accounts") || [];
  })();

  toast.promise(reqPromise, {
    loading: `正在验证账号`,
    success: `保存成功`,
    error: (err: any) => `添加失败, ${err.message}`,
  });
}

const handleDeleteAccount = async (name: string) => {
  const store = await load("settings.json", {
    defaults: {
      accounts: []
    },
    autoSave: true
  });

  const accounts = await store.get<any[]>("accounts") || [];
  accounts.find((account) => account.name === name) && accounts.splice(accounts.indexOf(name), 1);
  data.accounts = accounts;
  await store.set("accounts", accounts);
  toast.success(`删除账号 ${name} 成功`);
}

const handleEditAPI = async () => {
  const store = await load("settings.json", {
    defaults: {
      api_url: ""
    },
    autoSave: true
  });

  const reqPromise = (async () => {
    const res = await fetch(`${API_URL.value}`);

    if (!res.ok) throw new Error(`HTTP ${res.status}`);
    if (!(await res.text()).includes("HayFrp API")) {
      throw new Error("API 验证失败");
    }

    API_URL.value = data.api_url;
    await store.set("api_url", data.api_url);
  })();

  toast.promise(reqPromise, {
    loading: `正在验证 API`,
    success: `保存成功`,
    error: (err: any) => `保存成功, ${err.message}`,
  });
}

const handleEditFRPC = async () => {
  const store = await load("settings.json", {
    defaults: {
      frpc_path: "",
      frpc_version: "",
    },
    autoSave: true
  });

  const reqPromise = (async () => {
    const path = String.raw`${data.frpc_path}`

    const version = await invoke<string>("run_and_get_frpc", {path});

    if (version === undefined || version === "" || version === null) {
      throw new Error("未获取到版本信息");
    }

    data.frpc_version = version;
    data.frpc_path = path;
    FRPC_PATH.value = path;

    await store.set("frpc_version", version);
    await store.set("frpc_path", path);
  })();

  toast.promise(reqPromise, {
    loading: `正在验证 FRPC`,
    success: `保存成功`,
    error: (err: any) => `保存成功, ${err.message}`,
  });
}

const handleGetHomeName = (url: string) => {
  return (new URL(url)).hostname;
};

onMounted(async () => {
  const store = await load("settings.json", {
    defaults: {
      auto_tunnel: false,
      auto_tunnels: [],
      accounts: [],
      api_url: "https://api.hayfrp.com",
      frpc_version: "",
      frpc_path: "",
    },
    autoSave: true,
  });
  data.auto_start = await autostartIsEnabled();
  data.api_url = await store.get("api_url") || "https://api.hayfrp.com";
  data.auto_tunnel = await store.get("auto_tunnel") || false;
  data.accounts = await store.get("accounts") || [];
  data.frpc_version = await store.get("frpc_version") || "";
  data.frpc_path = await store.get("frpc_path") || "";

  watch(() => data.auto_tunnel, (value) => {
    store.set("auto_tunnel", value)
  });
});
</script>

<style>
a {
  position: static;
  font-weight: normal;
  text-decoration: none;
}

a::after {
  display: none;
}
</style>