import { useState } from 'react';
import { Eye, EyeOff, Key } from 'lucide-react';
import { useTranslation } from 'react-i18next';

interface ApiKeyInputProps {
  value: string;
  onChange: (value: string) => void;
  placeholder?: string;
}

export default function ApiKeyInput({ value, onChange, placeholder }: ApiKeyInputProps) {
  const { t } = useTranslation();
  const [show, setShow] = useState(false);

  return (
    <div className="relative">
      <div className="absolute left-2 top-1/2 -translate-y-1/2 text-muted-foreground">
        <Key size={12} />
      </div>
      <input
        type={show ? 'text' : 'password'}
        value={value}
        onChange={(e) => onChange(e.target.value)}
        placeholder={placeholder ?? t('builtin.apiKey')}
        className="w-full h-7 pl-7 pr-7 text-[12px] bg-card text-foreground rounded border border-input focus:border-ring outline-none transition-colors font-mono"
      />
      <button
        type="button"
        onClick={() => setShow(!show)}
        className="absolute right-1.5 top-1/2 -translate-y-1/2 text-muted-foreground hover:text-foreground p-0.5"
      >
        {show ? <EyeOff size={12} /> : <Eye size={12} />}
      </button>
    </div>
  );
}
