// 动态切换浏览器标签页 favicon（登录页图标 / 登录后图标）
export function setFavicon(href: string | null) {
  let link = document.querySelector<HTMLLinkElement>('link[rel="icon"]')
  if (!href) {
    // 未配置时回退到默认 favicon.svg
    link?.setAttribute('href', '/favicon.svg')
    return
  }
  if (!link) {
    link = document.createElement('link')
    link.rel = 'icon'
    document.head.appendChild(link)
  }
  link.setAttribute('href', href)
  link.setAttribute('type', '')
}
