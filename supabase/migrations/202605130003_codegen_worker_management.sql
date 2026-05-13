-- OpenPencil codegen worker management, provider rate limits, and circuit breakers.
-- Forward-only migration for Supabase Cloud.

create table if not exists public.codegen_provider_health (
  provider text primary key,
  total_requests integer not null default 0 check (total_requests >= 0),
  failed_requests integer not null default 0 check (failed_requests >= 0),
  consecutive_failures integer not null default 0 check (consecutive_failures >= 0),
  last_failure_type text,
  last_error text,
  circuit_open_until timestamptz,
  last_request_at timestamptz,
  updated_at timestamptz not null default now(),
  constraint codegen_provider_health_failure_type_check check (
    last_failure_type is null or last_failure_type in (
      'rate_limit',
      'timeout',
      'provider_error',
      'canceled',
      'execution_error'
    )
  )
);

create table if not exists public.codegen_provider_rate_windows (
  provider text not null,
  window_start timestamptz not null,
  request_count integer not null default 0 check (request_count >= 0),
  updated_at timestamptz not null default now(),
  primary key (provider, window_start)
);

create index if not exists codegen_provider_health_updated_idx
  on public.codegen_provider_health(updated_at desc);

create index if not exists codegen_provider_rate_windows_updated_idx
  on public.codegen_provider_rate_windows(updated_at desc);

alter table public.codegen_provider_health enable row level security;
alter table public.codegen_provider_rate_windows enable row level security;

drop policy if exists "codegen provider health is service role only" on public.codegen_provider_health;
create policy "codegen provider health is service role only"
on public.codegen_provider_health
for all
using (false)
with check (false);

drop policy if exists "codegen provider rate windows are service role only" on public.codegen_provider_rate_windows;
create policy "codegen provider rate windows are service role only"
on public.codegen_provider_rate_windows
for all
using (false)
with check (false);

create or replace function public.reserve_codegen_provider_capacity(
  p_provider text,
  p_max_per_minute integer default 30
)
returns boolean
language plpgsql
security definer
set search_path = public
as $$
declare
  v_window_start timestamptz;
  v_count integer;
begin
  if p_provider is null or length(trim(p_provider)) = 0 then
    return true;
  end if;

  v_window_start = date_trunc('minute', now());

  insert into public.codegen_provider_health(provider, total_requests, last_request_at, updated_at)
  values (p_provider, 0, now(), now())
  on conflict (provider)
  do update set
    last_request_at = excluded.last_request_at,
    updated_at = excluded.updated_at;

  if exists (
    select 1
    from public.codegen_provider_health
    where provider = p_provider
      and circuit_open_until is not null
      and circuit_open_until > now()
  ) then
    return false;
  end if;

  insert into public.codegen_provider_rate_windows(
    provider,
    window_start,
    request_count,
    updated_at
  )
  values (p_provider, v_window_start, 1, now())
  on conflict (provider, window_start)
  do update set
    request_count = public.codegen_provider_rate_windows.request_count + 1,
    updated_at = now()
  returning request_count into v_count;

  if v_count > greatest(p_max_per_minute, 1) then
    update public.codegen_provider_rate_windows
    set request_count = greatest(request_count - 1, 0),
        updated_at = now()
    where provider = p_provider
      and window_start = v_window_start;
    return false;
  end if;

  update public.codegen_provider_health
  set total_requests = total_requests + 1,
      last_request_at = now(),
      updated_at = now()
  where provider = p_provider;

  return true;
end;
$$;

revoke execute on function public.reserve_codegen_provider_capacity(text, integer)
  from public, anon, authenticated;

grant execute on function public.reserve_codegen_provider_capacity(text, integer)
  to service_role;
