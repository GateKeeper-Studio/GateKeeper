<script lang="ts">
	import { frameworkLibraries } from './framework-libraries';
</script>

<table>
	<thead>
		<tr>
			<th>Framework</th>
			<th>Package</th>
			<th>Client type</th>
			<th>Status</th>
			<th>What it owns</th>
		</tr>
	</thead>
	<tbody>
		{#each frameworkLibraries as library}
			<tr>
				<td><a href={library.docsHref}>{library.framework}</a></td>
				<td><code>{library.packageName}</code></td>
				<td>{library.clientType === 'confidential' ? 'Confidential (BFF)' : 'Public (SPA)'}</td>
				<td>{library.status === 'available' ? 'Available' : 'Planned'}</td>
				<td>{library.summary}</td>
			</tr>
		{/each}
	</tbody>
</table>
