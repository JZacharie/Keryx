"use client";

import React, { useState, useEffect } from "react";
import { 
  Plus, 
  Search, 
  Filter, 
  Zap, 
  Clock, 
  CheckCircle2, 
  AlertCircle,
  Link as LinkIcon,
  Play,
  Terminal,
  Server
} from "lucide-react";
import { motion, AnimatePresence } from "framer-motion";

// Components
const StatCard = ({ title, value, icon: Icon, trend, color }: any) => (
  <div className="glass rounded-2xl p-6 relative overflow-hidden group">
    <div className={`absolute top-0 right-0 w-24 h-24 bg-${color}/10 rounded-full blur-3xl -translate-y-1/2 translate-x-1/2 group-hover:bg-${color}/20 transition-all`} />
    <div className="flex items-center justify-between mb-4">
      <div className={`p-3 rounded-xl bg-${color}/10 text-${color}`}>
        <Icon className="w-6 h-6" />
      </div>
      {trend && (
        <span className="text-xs font-medium text-emerald-400 bg-emerald-400/10 px-2 py-1 rounded-full">
          {trend}
        </span>
      )}
    </div>
    <p className="text-slate-400 text-sm font-medium">{title}</p>
    <h3 className="text-3xl font-bold mt-1">{value}</h3>
  </div>
);

const StatusBadge = ({ status }: { status: string }) => {
  const configs: Record<string, { color: string, icon: any }> = {
    pending: { color: "slate", icon: Clock },
    processing: { color: "primary", icon: Zap },
    completed: { color: "emerald", icon: CheckCircle2 },
    failed: { color: "accent", icon: AlertCircle },
  };

  const config = configs[status.toLowerCase()] || configs.pending;
  const Icon = config.icon;

  return (
    <div className={`inline-flex items-center gap-1.5 px-2.5 py-1 rounded-full text-[11px] font-bold uppercase tracking-wider bg-${config.color}/10 text-${config.color} border border-${config.color}/20`}>
      <Icon className="w-3 h-3" />
      {status}
    </div>
  );
};

export default function DashboardPage() {
  const [activeJobs, setActiveJobs] = useState([]);
  const [isCreating, setIsCreating] = useState(false);
  const [formData, setFormData] = useState({ url: "", apiKey: "" });

  return (
    <div className="space-y-8 pb-10">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-4xl font-extrabold tracking-tight">Vue d'ensemble</h2>
          <p className="text-slate-400 font-medium">Monitoring du processing vidéo distribué</p>
        </div>
        <button 
          onClick={() => setIsCreating(true)}
          className="bg-gradient-primary hover:scale-105 transition-transform px-6 py-3 rounded-xl font-bold flex items-center gap-2 shadow-lg shadow-primary/20"
        >
          <Plus className="w-5 h-5" />
          Nouveau Job
        </button>
      </div>

      {/* Stats Grid */}
      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6">
        <StatCard title="Jobs Totaux" value="24" icon={Server} color="primary" trend="+12%" />
        <StatCard title="En cours" value="3" icon={Zap} color="secondary" />
        <StatCard title="Taux de succès" value="98.2%" icon={CheckCircle2} color="emerald" trend="+2%" />
        <StatCard title="Temps moyen" value="4.2m" icon={Clock} color="slate" />
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-3 gap-8">
        {/* Main Jobs List */}
        <div className="lg:col-span-2 space-y-6">
          <div className="flex items-center justify-between mb-2">
            <h3 className="text-xl font-bold flex items-center gap-2">
              <Activity className="text-primary w-5 h-5" />
              Jobs Récents
            </h3>
            <div className="flex items-center gap-2">
              <div className="relative">
                <Search className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-slate-500" />
                <input 
                  type="text" 
                  placeholder="Rechercher..." 
                  className="bg-white/5 border border-slate-700/50 rounded-lg pl-9 pr-4 py-1.5 text-sm focus:outline-none focus:ring-2 focus:ring-primary/50 transition-all w-48"
                />
              </div>
              <button className="p-2 glass-light rounded-lg hover:bg-white/10 transition-colors">
                <Filter className="w-4 h-4 text-slate-400" />
              </button>
            </div>
          </div>

          <div className="space-y-4">
            {[1, 2, 3].map((i) => (
              <motion.div 
                key={i}
                initial={{ opacity: 0, y: 20 }}
                animate={{ opacity: 1, y: 0 }}
                transition={{ delay: i * 0.1 }}
                className="glass hover:bg-white/[0.04] transition-all p-5 rounded-2xl border border-white/5 group cursor-pointer"
              >
                <div className="flex items-center gap-4">
                  <div className="w-16 h-16 rounded-xl bg-slate-800 flex items-center justify-center overflow-hidden border border-slate-700 relative">
                    <img src={`https://picsum.photos/seed/${i+42}/200/200`} alt="thumbnail" className="object-cover w-full h-full opacity-60 group-hover:scale-110 transition-transform" />
                    <div className="absolute inset-0 flex items-center justify-center">
                      <Play className="w-6 h-6 text-white/50 group-hover:text-white transition-colors" />
                    </div>
                  </div>
                  <div className="flex-1 min-w-0">
                    <div className="flex items-center gap-3 mb-1">
                      <h4 className="font-bold truncate text-lg">Analyse Stratégique - Marketing Q4</h4>
                      <StatusBadge status={i === 1 ? "processing" : "completed"} />
                    </div>
                    <div className="flex items-center gap-4 text-xs text-slate-400 font-medium">
                      <span className="flex items-center gap-1">
                        <LinkIcon className="w-3 h-3" />
                        youtube.com/watch?v=...
                      </span>
                      <span className="flex items-center gap-1">
                        <Clock className="w-3 h-3" />
                        Il y a {i * 2} heures
                      </span>
                    </div>
                  </div>
                  <div className="text-right">
                    <div className="text-sm font-bold text-slate-200">12.4 MB</div>
                    <div className="text-[10px] text-slate-500 uppercase font-black mt-2">ID: {Math.random().toString(36).substr(2, 8)}</div>
                  </div>
                  <div className="ml-4 opacity-0 group-hover:opacity-100 transition-opacity">
                    <ChevronRight className="w-5 h-5 text-slate-600" />
                  </div>
                </div>
                {i === 1 && (
                  <div className="mt-4 pt-4 border-t border-white/5">
                    <div className="flex items-center justify-between text-xs mb-2">
                      <span className="text-primary font-bold">Transcription en cours...</span>
                      <span className="text-slate-400">65%</span>
                    </div>
                    <div className="h-1.5 w-full bg-slate-800 rounded-full overflow-hidden">
                      <motion.div 
                        initial={{ width: 0 }}
                        animate={{ width: "65%" }}
                        className="h-full bg-gradient-primary"
                      />
                    </div>
                  </div>
                )}
              </motion.div>
            ))}
          </div>
        </div>

        {/* Sidebar Widgets */}
        <div className="space-y-8">
          {/* Node Health */}
          <div className="glass rounded-2xl p-6">
            <h3 className="text-lg font-bold mb-4 flex items-center gap-2">
              <Activity className="text-secondary w-5 h-5" />
              État des Noeuds
            </h3>
            <div className="space-y-4">
              {[
                { name: "SVD Engine (GPU)", status: "healthy", load: "78%" },
                { name: "Whisper ASR", status: "healthy", load: "12%" },
                { name: "Dewatermark CV", status: "busy", load: "94%" },
                { name: "Orchestrator", status: "healthy", load: "5%" },
              ].map((node) => (
                <div key={node.name} className="flex items-center justify-between">
                  <div className="text-sm font-medium">{node.name}</div>
                  <div className="flex items-center gap-3">
                    <div className="text-[10px] text-slate-500 font-bold">{node.load}</div>
                    <div className={`w-2 h-2 rounded-full ${node.status === 'healthy' ? 'bg-emerald-500 shadow-[0_0_8px_rgba(16,185,129,0.5)]' : 'bg-amber-500 shadow-[0_0_8px_rgba(245,158,11,0.5)]'}`} />
                  </div>
                </div>
              ))}
            </div>
          </div>

          {/* Quick Terminal */}
          <div className="glass rounded-2xl p-6 border-l-4 border-primary/40">
            <h3 className="text-lg font-bold mb-4 flex items-center gap-2">
              <Terminal className="text-primary w-5 h-5" />
              Live Logs
            </h3>
            <div className="font-mono text-[10px] space-y-1 h-48 overflow-y-auto text-slate-300">
              <p className="text-slate-500">[10:45:02] Initializing cluster connect...</p>
              <p className="text-emerald-500">[10:45:03] Redis pipeline active.</p>
              <p className="text-slate-300">[10:45:04] Job 872a1bc: Phase 1 started.</p>
              <p className="text-slate-300">[10:45:10] Job 872a1bc: Downloading 48.2MB...</p>
              <p className="text-secondary">[10:45:22] Extractor: Success.</p>
              <p className="text-slate-300">[10:45:23] Job 872a1bc: Starting Whisper STT...</p>
              <p className="text-slate-500 animate-pulse">_</p>
            </div>
          </div>
        </div>
      </div>

      {/* Creation Modal */}
      <AnimatePresence>
        {isCreating && (
          <div className="fixed inset-0 z-[100] flex items-center justify-center p-4">
            <motion.div 
              initial={{ opacity: 0 }}
              animate={{ opacity: 1 }}
              exit={{ opacity: 0 }}
              onClick={() => setIsCreating(false)}
              className="absolute inset-0 bg-black/60 backdrop-blur-md"
            />
            <motion.div 
              initial={{ scale: 0.9, opacity: 0, y: 20 }}
              animate={{ scale: 1, opacity: 1, y: 0 }}
              exit={{ scale: 0.9, opacity: 0, y: 20 }}
              className="glass max-w-lg w-full p-8 rounded-3xl relative z-10 border border-white/10"
            >
              <h3 className="text-2xl font-bold mb-2">Lancer un nouveau Job</h3>
              <p className="text-slate-400 text-sm mb-6">Collez l'URL de la vidéo source (YouTube, Twitter, etc.)</p>
              
              <div className="space-y-4">
                <div>
                  <label className="text-xs font-bold text-slate-500 uppercase tracking-widest mb-2 block">Source URL</label>
                  <div className="relative group">
                    <LinkIcon className="absolute left-4 top-1/2 -translate-y-1/2 w-5 h-5 text-slate-500 group-focus-within:text-primary transition-colors" />
                    <input 
                      type="text" 
                      placeholder="https://youtube.com/..." 
                      className="w-full bg-slate-900/50 border border-slate-700/50 rounded-xl pl-12 pr-4 py-4 text-slate-200 focus:outline-none focus:ring-2 focus:ring-primary/50 transition-all font-medium"
                    />
                  </div>
                </div>

                <div>
                  <label className="text-xs font-bold text-slate-500 uppercase tracking-widest mb-2 block">API Key</label>
                  <input 
                    type="password" 
                    placeholder="Votre clé Bearer" 
                    className="w-full bg-slate-900/50 border border-slate-700/50 rounded-xl px-4 py-3 text-slate-200 focus:outline-none focus:ring-2 focus:ring-primary/50 transition-all text-sm"
                  />
                </div>

                <div className="pt-4 flex gap-3">
                  <button 
                    onClick={() => setIsCreating(false)}
                    className="flex-1 px-4 py-3 rounded-xl bg-white/5 hover:bg-white/10 font-bold transition-all"
                  >
                    Annuler
                  </button>
                  <button className="flex-1 px-4 py-3 rounded-xl bg-gradient-primary font-bold shadow-lg shadow-primary/20 hover:scale-[1.02] active:scale-[0.98] transition-all">
                    Démarrer le Traitement
                  </button>
                </div>
              </div>
            </motion.div>
          </div>
        )}
      </AnimatePresence>
    </div>
  );
}

function ChevronRight(props: any) {
  return (
    <svg
      {...props}
      xmlns="http://www.w3.org/2000/svg"
      width="24"
      height="24"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      strokeLinecap="round"
      strokeLinejoin="round"
    >
      <path d="m9 18 6-6-6-6" />
    </svg>
  );
}

function Activity(props: any) {
  return (
    <svg
      {...props}
      xmlns="http://www.w3.org/2000/svg"
      width="24"
      height="24"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      strokeLinecap="round"
      strokeLinejoin="round"
    >
      <path d="M22 12h-2.48a2 2 0 0 0-1.93 1.46l-2.35 8.36a.25.25 0 0 1-.48 0L9.24 2.18a.25.25 0 0 0-.48 0l-2.35 8.36A2 2 0 0 1 4.48 12H2" />
    </svg>
  );
}
