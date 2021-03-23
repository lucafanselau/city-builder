use tasks::task_pool::TaskPool;

fn fib(i: usize) -> usize {
    match i {
        0 | 1 => 1,
        _ => fib(i - 1) + fib(i - 2),
    }
}

fn main() {
    {
        let pool = TaskPool::new(None, "scoping_pool");
        let fib = pool.scope(|scope| {
            (20..50)
                .into_iter()
                .rev()
                .for_each(|i| scope.spawn(async move { fib(i) }));
        });
        println!("{:#?}", fib);
    }
}
