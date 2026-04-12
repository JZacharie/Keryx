"use client";

import React from "react";
import Link from "next/link";
import { usePathname } from "next/navigation";
import { 
  LayoutDashboard, 
  Video, 
  History, 
  Settings, 
  ShieldCheck, 
  Activity,
  ChevronRight,
  Monitor
} from "lucide-react";
import { clsx, type ClassValue } from "clsx";
import { twMerge } from "tailwind-merge";

function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs));
}

const navItems = [
  { name: "Dashboard", href: "/", icon: LayoutDashboard },
  { name: "Create Job", href: "/create", icon: Video },
  { name: "History", href: "/history", icon: History },
  { name: "Node Status", href: "/nodes", icon: Monitor },
  { name: "Security", href: "/security", icon: ShieldCheck },
  { name: "Settings", href: "/settings", icon: Settings },
];

export function Sidebar() {
  const pathname = usePathname();

  return (
    <div className="w-64 h-screen glass border-r border-border flex flex-col p-4 z-50 overflow-hidden">
      <div className="flex items-center gap-3 px-2 mb-10">
        <div className="w-10 h-10 rounded-xl bg-gradient-primary flex items-center justify-center shadow-lg shadow-primary/20">
          <Activity className="text-white w-6 h-6" />
        </div>
        <div>
          <h1 className="text-xl font-bold tracking-tight text-gradient">KERYX</h1>
          <p className="text-[10px] text-slate-500 font-medium tracking-widest uppercase">Orchestrator v0.5</p>
        </div>
      </div>

      <nav className="flex-1 space-y-1">
        {navItems.map((item) => {
          const isActive = pathname === item.href;
          return (
            <Link
              key={item.href}
              href={item.href}
              className={cn(
                "flex items-center gap-3 px-3 py-2.5 rounded-lg transition-all duration-200 group relative",
                isActive 
                  ? "bg-primary/10 text-primary font-semibold" 
                  : "text-slate-400 hover:text-slate-100 hover:bg-white/5"
              )}
            >
              <item.icon className={cn(
                "w-5 h-5 transition-transform duration-200",
                isActive ? "scale-110" : "group-hover:scale-110"
              )} />
              <span className="text-sm">{item.name}</span>
              {isActive && (
                <div className="absolute right-2 w-1 h-5 bg-primary rounded-full" />
              )}
            </Link>
          );
        })}
      </nav>

      <div className="mt-auto pt-6 border-t border-border">
        <div className="glass-light rounded-xl p-3 flex items-center gap-3">
          <div className="w-8 h-8 rounded-full bg-slate-800 border border-slate-700 flex items-center justify-center overflow-hidden">
            <span className="text-xs font-bold text-slate-400">JZ</span>
          </div>
          <div className="flex-1 overflow-hidden">
            <p className="text-xs font-medium truncate">Joseph Zacharie</p>
            <p className="text-[10px] text-emerald-500 flex items-center gap-1">
              <span className="w-1 h-1 bg-emerald-500 rounded-full animate-pulse" />
              Connected
            </p>
          </div>
        </div>
      </div>
    </div>
  );
}
