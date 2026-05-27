import { useEffect, useMemo, useState, type CSSProperties, type ReactNode } from 'react';
import { useNavigate } from '@tanstack/react-router';
import {
  Bell,
  ClipboardList,
  Cloud,
  Database,
  FileClock,
  Home,
  LogOut,
  Moon,
  PenTool,
  Plus,
  Search,
  ServerCog,
  Settings2,
  ShieldCheck,
  Sun,
  type LucideIcon,
} from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { Button } from '@/components/ui/button';
import {
  CommandDialog,
  CommandEmpty,
  CommandGroup,
  CommandInput,
  CommandItem,
  CommandList,
} from '@/components/ui/command';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuGroup,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';
import {
  Sidebar,
  SidebarContent,
  SidebarFooter,
  SidebarGroup,
  SidebarGroupContent,
  SidebarGroupLabel,
  SidebarHeader,
  SidebarInset,
  SidebarMenu,
  SidebarMenuButton,
  SidebarMenuItem,
  SidebarProvider,
  SidebarTrigger,
} from '@/components/ui/sidebar';
import { cn } from '@/lib/utils';
import { useCloudAuthStore } from '@/stores/cloud-auth-store';
import { appStorage, initAppStorage } from '@/utils/app-storage';

export type WorkbenchNavKey =
  | 'home'
  | 'cloud'
  | 'tasks'
  | 'notifications'
  | 'workers'
  | 'providers'
  | 'audit';

interface WorkbenchShellProps {
  active: WorkbenchNavKey;
  title: string;
  description?: string;
  children: ReactNode;
  notificationCount?: number;
  onCreateDesign?: () => void;
  createDisabled?: boolean;
  toolbar?: ReactNode;
  contentClassName?: string;
}

interface NavItem {
  key: WorkbenchNavKey;
  labelKey: string;
  descriptionKey: string;
  icon: LucideIcon;
  to: string;
  search?: Record<string, string>;
}

const THEME_STORAGE_KEY = 'openpencil-theme';

const mainNav: NavItem[] = [
  {
    key: 'home',
    labelKey: 'workbench.nav.home',
    descriptionKey: 'workbench.command.home',
    icon: Home,
    to: '/',
  },
  {
    key: 'cloud',
    labelKey: 'workbench.nav.cloud',
    descriptionKey: 'workbench.command.cloud',
    icon: Cloud,
    to: '/cloud',
  },
  {
    key: 'tasks',
    labelKey: 'workbench.nav.tasks',
    descriptionKey: 'workbench.command.tasks',
    icon: ClipboardList,
    to: '/tasks',
  },
  {
    key: 'notifications',
    labelKey: 'workbench.nav.notifications',
    descriptionKey: 'workbench.command.notifications',
    icon: Bell,
    to: '/tasks',
    search: { view: 'notifications' },
  },
];

const opsNav: NavItem[] = [
  {
    key: 'workers',
    labelKey: 'workbench.nav.workers',
    descriptionKey: 'workbench.command.workers',
    icon: ServerCog,
    to: '/tasks/workers',
    search: { view: 'workers' },
  },
  {
    key: 'providers',
    labelKey: 'workbench.nav.providers',
    descriptionKey: 'workbench.command.providers',
    icon: Settings2,
    to: '/tasks/workers',
    search: { view: 'providers' },
  },
  {
    key: 'audit',
    labelKey: 'workbench.nav.audit',
    descriptionKey: 'workbench.command.audit',
    icon: FileClock,
    to: '/tasks/workers',
    search: { view: 'audit' },
  },
];

function navTarget(item: NavItem) {
  return item.search ? ({ to: item.to, search: item.search } as const) : ({ to: item.to } as const);
}

function useWorkbenchTheme() {
  const [theme, setTheme] = useState<'dark' | 'light'>('dark');

  useEffect(() => {
    const restore = async () => {
      await initAppStorage();
      const saved = appStorage.getItem(THEME_STORAGE_KEY);
      document.documentElement.classList.remove('dark');
      if (saved === 'light') {
        document.documentElement.classList.add('light');
        setTheme('light');
      } else {
        document.documentElement.classList.remove('light');
        setTheme('dark');
      }
    };
    void restore();
  }, []);

  const toggleTheme = () => {
    const next = theme === 'dark' ? 'light' : 'dark';
    document.documentElement.classList.remove('dark');
    document.documentElement.classList.toggle('light', next === 'light');
    appStorage.setItem(THEME_STORAGE_KEY, next);
    setTheme(next);
  };

  return { theme, toggleTheme };
}

export function WorkbenchShell({
  active,
  title,
  description,
  children,
  notificationCount = 0,
  onCreateDesign,
  createDisabled,
  toolbar,
  contentClassName,
}: WorkbenchShellProps) {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const [commandOpen, setCommandOpen] = useState(false);
  const { theme, toggleTheme } = useWorkbenchTheme();
  const allNavItems = useMemo(() => [...mainNav, ...opsNav], []);

  const runCommand = (item: NavItem) => {
    setCommandOpen(false);
    void navigate(navTarget(item));
  };

  const createDesign = () => {
    if (onCreateDesign) {
      onCreateDesign();
      return;
    }
    void navigate({ to: '/cloud' });
  };

  return (
    <SidebarProvider
      className="h-screen overflow-hidden bg-background text-foreground"
      style={{ '--sidebar-width': '14rem' } as CSSProperties}
    >
      <Sidebar collapsible="offcanvas" className="border-r border-border bg-card">
        <SidebarHeader className="h-10 justify-center border-b border-border px-2 py-0">
          <button
            type="button"
            className="flex min-w-0 items-center gap-2 rounded px-1 py-1 text-left hover:bg-accent/50"
            onClick={() => void navigate({ to: '/' })}
          >
            <span className="flex size-7 items-center justify-center rounded border border-border bg-background/60 text-muted-foreground">
              <PenTool />
            </span>
            <span className="min-w-0">
              <span className="block truncate text-xs font-semibold">OpenPencil</span>
              <span className="block truncate text-[11px] text-muted-foreground">
                {t('workbench.productLine')}
              </span>
            </span>
          </button>
        </SidebarHeader>
        <SidebarContent>
          <WorkbenchNavGroup active={active} items={mainNav} label={t('workbench.nav.workspace')} />
          <WorkbenchNavGroup active={active} items={opsNav} label={t('workbench.nav.operations')} />
        </SidebarContent>
        <SidebarFooter className="border-t border-border p-2">
          <div className="rounded border border-border bg-background/50 px-2 py-2">
            <div className="flex items-center gap-2 text-xs font-medium">
              <Database />
              {t('workbench.queueHealth')}
            </div>
            <p className="mt-1 text-[11px] leading-4 text-muted-foreground">{t('workbench.queueHint')}</p>
          </div>
        </SidebarFooter>
      </Sidebar>

      <SidebarInset className="w-auto min-w-0 overflow-hidden">
        <header className="sticky top-0 z-20 flex h-10 items-center gap-2 border-b bg-card px-2 backdrop-blur">
          <SidebarTrigger className="md:hidden" />
          <div className="min-w-0 flex-1">
            <div className="flex items-center gap-2">
              <h1 className="truncate text-xs font-semibold">{title}</h1>
              {description && (
                <span className="hidden truncate text-xs text-muted-foreground lg:inline">
                  {description}
                </span>
              )}
            </div>
          </div>
          <Button
            variant="outline"
            size="sm"
            className="hidden h-7 w-56 justify-start bg-background/60 px-2 text-xs text-muted-foreground shadow-none md:flex"
            onClick={() => setCommandOpen(true)}
          >
            <Search data-icon="inline-start" />
            {t('workbench.command.placeholder')}
          </Button>
          <Button
            variant="ghost"
            size="icon-sm"
            className="md:hidden"
            onClick={() => setCommandOpen(true)}
            aria-label={t('workbench.command.open')}
          >
            <Search />
          </Button>
          <Button
            variant="ghost"
            size="icon-sm"
            aria-label={t('workbench.notifications')}
            onClick={() => void navigate({ to: '/tasks', search: { view: 'notifications' } })}
          >
            <Bell />
            {notificationCount > 0 && (
              <span className="absolute -mt-5 ml-5 flex h-4 min-w-4 items-center justify-center rounded-full bg-primary px-1 text-[10px] text-primary-foreground">
                {notificationCount}
              </span>
            )}
          </Button>
          <Button
            variant="ghost"
            size="icon-sm"
            aria-label={theme === 'dark' ? t('topbar.lightMode') : t('topbar.darkMode')}
            onClick={toggleTheme}
          >
            {theme === 'dark' ? <Sun /> : <Moon />}
          </Button>
          <Button
            size="sm"
            className="h-7 px-2 text-xs"
            onClick={createDesign}
            disabled={createDisabled}
          >
            <Plus data-icon="inline-start" />
            {t('workbench.newDesign')}
          </Button>
          <WorkbenchUserMenu />
        </header>

        <main className={cn('min-w-0 flex-1 overflow-auto p-2', contentClassName)}>
          {toolbar}
          {children}
        </main>
      </SidebarInset>

      <CommandDialog
        open={commandOpen}
        onOpenChange={setCommandOpen}
        title={t('workbench.command.title')}
        description={t('workbench.command.description')}
      >
        <CommandInput placeholder={t('workbench.command.placeholder')} />
        <CommandList>
          <CommandEmpty>{t('workbench.command.empty')}</CommandEmpty>
          <CommandGroup heading={t('workbench.command.group')}>
            {allNavItems.map((item) => {
              const Icon = item.icon;
              return (
                <CommandItem key={item.key} value={t(item.labelKey)} onSelect={() => runCommand(item)}>
                  <Icon />
                  <span>{t(item.labelKey)}</span>
                  <span className="ml-auto text-xs text-muted-foreground">
                    {t(item.descriptionKey)}
                  </span>
                </CommandItem>
              );
            })}
          </CommandGroup>
        </CommandList>
      </CommandDialog>
    </SidebarProvider>
  );
}

function WorkbenchNavGroup({
  active,
  items,
  label,
}: {
  active: WorkbenchNavKey;
  items: NavItem[];
  label: string;
}) {
  const { t } = useTranslation();
  const navigate = useNavigate();

  return (
    <SidebarGroup className="p-2">
      <SidebarGroupLabel className="h-7 px-1 text-[11px] tracking-wider">{label}</SidebarGroupLabel>
      <SidebarGroupContent>
        <SidebarMenu>
          {items.map((item) => {
            const Icon = item.icon;
            return (
              <SidebarMenuItem key={item.key}>
                <SidebarMenuButton
                  asChild
                  isActive={active === item.key}
                  tooltip={t(item.labelKey)}
                  className="h-7 rounded text-xs"
                >
                  <button type="button" onClick={() => void navigate(navTarget(item))}>
                    <Icon />
                    <span>{t(item.labelKey)}</span>
                  </button>
                </SidebarMenuButton>
              </SidebarMenuItem>
            );
          })}
        </SidebarMenu>
      </SidebarGroupContent>
    </SidebarGroup>
  );
}

function WorkbenchUserMenu() {
  const { t } = useTranslation();
  const user = useCloudAuthStore((s) => s.user);
  const signOut = useCloudAuthStore((s) => s.signOut);

  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button
          variant="outline"
          size="icon-sm"
          className="bg-background/60 shadow-none"
          aria-label={t('workbench.userMenu')}
        >
          <span className="text-xs font-semibold">OP</span>
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end" className="w-48">
        <DropdownMenuLabel>
          <span className="block">{t('workbench.userMenu')}</span>
          {user?.email && (
            <span className="block truncate text-xs font-normal text-muted-foreground">
              {user.email}
            </span>
          )}
        </DropdownMenuLabel>
        <DropdownMenuSeparator />
        <DropdownMenuGroup>
          <DropdownMenuItem>
            <ShieldCheck />
            {t('workbench.accountStatus')}
          </DropdownMenuItem>
          <DropdownMenuItem onClick={() => void signOut()}>
            <LogOut />
            {t('auth.signOut')}
          </DropdownMenuItem>
        </DropdownMenuGroup>
      </DropdownMenuContent>
    </DropdownMenu>
  );
}
