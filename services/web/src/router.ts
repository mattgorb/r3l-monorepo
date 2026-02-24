import { createRouter, createWebHistory } from 'vue-router'
import Home from './pages/Home.vue'
import Verify from './pages/Verify.vue'
import Search from './pages/Search.vue'
import Prove from './pages/Prove.vue'
import Docs from './pages/Docs.vue'
import Account from './pages/Account.vue'

const router = createRouter({
  history: createWebHistory(),
  routes: [
    { path: '/', component: Home },
    { path: '/verify', component: Verify },
    { path: '/identity', redirect: '/verify' },
    { path: '/search', component: Search },
    { path: '/search/:hash', component: Search },
    // Redirects for old routes
    { path: '/lookup', redirect: '/search' },
    { path: '/lookup/:hash', redirect: to => `/search/${to.params.hash}` },
    { path: '/similar', redirect: '/search' },
    { path: '/prove', component: Prove },
    { path: '/docs', component: Docs },
    { path: '/account', component: Account },
    { path: '/org', redirect: '/account' },
  ],
})

export default router
