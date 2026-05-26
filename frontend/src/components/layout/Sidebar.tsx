import { Link, useLocation } from 'react-router-dom'
import {
  LayoutDashboard,
  Radio,
  Layers,
  Key,
  BarChart3,
  Settings,
} from 'lucide-react'
import { cn } from '@/lib/utils'

const navItems = [
  {
    title: '仪表盘',
    href: '/',
    icon: LayoutDashboard,
  },
  {
    title: '渠道管理',
    href: '/channels',
    icon: Radio,
  },
  {
    title: '分组管理',
    href: '/groups',
    icon: Layers,
  },
  {
    title: 'API Keys',
    href: '/api-keys',
    icon: Key,
  },
  {
    title: '统计分析',
    href: '/stats',
    icon: BarChart3,
  },
]

export function Sidebar() {
  const location = useLocation()

  return (
    <aside className="w-64 border-r bg-sidebar">
      <div className="flex h-16 items-center border-b px-6">
        <Link to="/" className="flex items-center gap-2 font-semibold">
          <Settings className="h-6 w-6" />
          <span>Galaxy Proxy</span>
        </Link>
      </div>
      <nav className="space-y-1 p-4">
        {navItems.map((item) => {
          const isActive =
            item.href === '/'
              ? location.pathname === '/'
              : location.pathname.startsWith(item.href)

          return (
            <Link
              key={item.href}
              to={item.href}
              className={cn(
                'flex items-center gap-3 rounded-lg px-3 py-2 text-sm transition-colors',
                isActive
                  ? 'bg-sidebar-accent text-sidebar-accent-foreground'
                  : 'text-sidebar-foreground hover:bg-sidebar-accent/50'
              )}
            >
              <item.icon className="h-4 w-4" />
              <span>{item.title}</span>
            </Link>
          )
        })}
      </nav>
    </aside>
  )
}
