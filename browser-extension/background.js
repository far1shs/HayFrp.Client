// Background Service Worker

console.log('[HayFrp Helper] Background service worker started');

// 监听扩展安装
chrome.runtime.onInstalled.addListener(() => {
  console.log('[HayFrp Helper] Extension installed');
  
  // 设置默认配置
  chrome.storage.sync.get(['serverUrl'], (result) => {
    if (!result.serverUrl) {
      chrome.storage.sync.set({
        serverUrl: 'http://127.0.0.1:3737',
        apiKey: ''
      });
    }
  });
});

// 监听来自 content script 的消息
chrome.runtime.onMessage.addListener((request, sender, sendResponse) => {
  console.log('[HayFrp Helper] Background received message:', request);
  
  // 这里可以添加后台处理逻辑
  
  return true;
});
