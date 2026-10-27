import {
  MessageCircle,
  Hash,
  SquareCheck,
  ShieldCheck,
  FileText,
  Activity,
  Search,
  Plus,
  ArrowUp,
  X,
  PanelRight,
  Paperclip,
  Check,
  SlidersHorizontal,
  ChevronRight,
  Users,
  Layers,
  Sun,
  Moon,
  BookOpen,
  Download,
  KeyRound,
  LockKeyhole,
  Sparkles,
  BrainCircuit,
  CalendarDays,
  Settings,
  RefreshCw,
  Unplug,
} from "lucide-react";
export type Agent = "Janet" | "Harley";
const icons = {
  chat: MessageCircle,
  thread: Hash,
  tasks: SquareCheck,
  shield: ShieldCheck,
  file: FileText,
  activity: Activity,
  search: Search,
  plus: Plus,
  arrow: ArrowUp,
  close: X,
  panel: PanelRight,
  attach: Paperclip,
  check: Check,
  settings: SlidersHorizontal,
  gear: Settings,
  refresh: RefreshCw,
  disconnect: Unplug,
  chevron: ChevronRight,
  agents: Users,
  layers: Layers,
  sun: Sun,
  moon: Moon,
  skills: BookOpen,
  download: Download,
  key: KeyRound,
  lock: LockKeyhole,
  soul: Sparkles,
  memory: BrainCircuit,
  calendar: CalendarDays,
};
export function Icon({ name }: { name: keyof typeof icons }) {
  const Component = icons[name];
  return <Component size={20} strokeWidth={1.65} aria-hidden="true" />;
}
export function Avatar({
  agent,
  small = false,
}: {
  agent: Agent;
  small?: boolean;
}) {
  return (
    <span
      aria-hidden="true"
      className={`avatar ${agent.toLowerCase()} ${small ? "small" : ""}`}
    >
      <img
        src={`/prototype/agents/${agent.toLowerCase()}.png`}
        alt=""
        width="48"
        height="48"
      />
    </span>
  );
}
export const starters: Record<
  Agent,
  { user: boolean; text: string; card?: boolean }[]
> = {
  Janet: [
    {
      user: true,
      text: "Hey Janet, give me a quick update on our priorities this week.",
    },
    {
      user: false,
      text: "Good morning, User! Everything is organized. Three things could use your attention today:\n\n1. Finalize the v0.2 release plan — Wednesday\n2. Review infrastructure costs — Thursday\n3. Prepare the customer demo — Friday",
    },
    {
      user: false,
      text: "The release plan is in good shape. I’ve pulled everything into a draft, with two decisions for you to review.",
      card: true,
    },
    {
      user: true,
      text: "Thanks. Let’s make sure we’re ready for the customer demo, too.",
    },
    {
      user: false,
      text: "Already on my radar. The agenda is drafted and the demo environment is next. I’d suggest a 45-minute prep session on Thursday — I can prepare an invite for your review.",
    },
  ],
  Harley: [
    { user: true, text: "Hey Harley, what’s on my radar for the weekend?" },
    {
      user: false,
      text: "Okay, weekend mission: less admin, more adventure. Saturday is wide open. I’ve shortlisted a few mountain trails — scenic, snack-friendly, and only mildly ambitious.",
    },
    {
      user: false,
      text: "The mountain escape plan is drafted. Snacks are non-negotiable. The groceries can wait until Friday; the car appointment is the only thing I’d book sooner.",
      card: true,
    },
  ],
};
