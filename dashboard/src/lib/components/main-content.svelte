<script lang="ts">
  import ChevronLeftIcon from '@lucide/svelte/icons/chevron-left';

  type Props = {
    children: any;
    title?: string;
    description?: string;
    goBack?: {
      message: string;
      href: string;
    }
  }

  let { children, goBack, title, description }: Props = $props();
</script>

<main class="flex flex-col p-4">
  {#if goBack}
    <a
      href={goBack.href}
      class="text-md mb-4 flex w-fit items-center gap-2 text-gray-600 hover:text-gray-800 hover:underline dark:text-gray-300 hover:dark:text-gray-500"
    >
      <ChevronLeftIcon size={24} />
      {goBack.message}
    </a>
  {/if}

  {#if title}
    <h1 class="text-3xl font-bold tracking-tight">{title}</h1>
  {/if}

  {#if description}
    <span class="mt-3 text-sm tracking-tight text-gray-600 dark:text-gray-300">
      {description}
    </span>
  {/if}

	{@render children()}
</main>