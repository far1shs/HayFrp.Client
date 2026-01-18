<template>
  <div class="flex flex-col gap-3 h-[calc(100vh-108px)]">
    <div class="text-sm text-gray-500 dark:text-gray-400 text-center">
      {{ hitokoto }}
    </div>

    <Item @click="handleSign" variant="outline" class="bg-background dark:bg-[#151515]" as-child>
      <a href="#">
        <ItemMedia>
          <CalendarRangeIcon class="size-5"/>
        </ItemMedia>
        <ItemContent>
          <ItemTitle>签到</ItemTitle>
        </ItemContent>
      </a>
    </Item>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue';
import {Item, ItemContent, ItemMedia, ItemTitle} from "@/components/ui/item";
import {CalendarRangeIcon} from "lucide-vue-next";
import {load} from "@tauri-apps/plugin-store";
import {API_URL} from "@/lib/env.ts";
import {toast} from "vue-sonner";
import {invoke} from "@tauri-apps/api/core";

const hitokoto = ref('加载中...');
const getHitokoto = async () => {
  try {
    const response = await fetch('https://v1.hitokoto.cn');
    const data = await response.json();
    hitokoto.value = data.hitokoto + ' —— ' + data.from;
  } catch (error) {
    hitokoto.value = '获取一言失败：' + error;
  }
};
onMounted(() => {
  getHitokoto();
});

const handleSign = async () => {
  const store = await load("settings.json", {
    defaults: {
      accounts: []
    }
  });

  const accounts = (await store.get<any[]>("accounts")) || [];

  if (accounts.length === 0) {
    toast.warning("请先到设置添加账号");
    return;
  }

  const fetchPromises = accounts.map(async (account, index) => {
    try {
      const csrf: string = await invoke("secure_decrypt", {
        encryptedBase64: account.csrf
      });

      const req = await fetch(`${API_URL.value}/user`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          type: "sign",
          csrf: csrf
        })
      });

      if (!req.ok) throw new Error(`${req.status} ${req.statusText}`);

      const json = await req.json();
      if (json.status !== 200) throw new Error(json.message);

      accounts[index].tunnels = json.proxies || [];
      accounts[index].error = false;
      return { status: "fulfilled", name: account.name };

    } catch (e: any) {
      accounts[index].error = true;
      accounts[index].errorMessage = e.message || "未知错误";
      return Promise.reject({ name: account.name, message: e.message });
    } finally {
      accounts[index].loading = false;
    }
  });

  const promise = Promise.allSettled(fetchPromises).then(results => {
    const fails = results
        .filter(r => r.status === "rejected")
        .map((r: any) => r.reason.name);

    if (fails.length > 0) {
      throw new Error(`签到失败：${fails.join(",")}`);
    }

    return results;
  });

  toast.promise(promise, {
    loading: "正在签到",
    success: "签到成功",
    error: (err: any) => err.message,
  });
};
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
