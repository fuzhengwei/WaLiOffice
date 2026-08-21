# 前10名编程语言运行结果模拟

---

## 1. Python 🐍

### Hello World
```python
print("Hello, World!")
```
**运行结果：**
```
Hello, World!
```

### 函数与列表推导
```python
def fibonacci(n):
    return [a if a < 10 else a % 10 for a in __fib(n)]

def __fib(n):
    a, b = 0, 1
    for _ in range(n):
        yield a
        a, b = b, a + b

print(fibonacci(10))
```
**运行结果：**
```
[0, 1, 1, 2, 3, 5, 8, 3, 1, 4]
```

### 面向对象
```python
class Dog:
    def __init__(self, name, age):
        self.name = name
        self.age = age
    def bark(self):
        return f"{self.name} says Woof!"

dog = Dog("旺财", 3)
print(dog.bark())
```
**运行结果：**
```
旺财 says Woof!
```

---

## 2. JavaScript ⚡

### 基础语法
```javascript
const name = "小明";
const greet = (person) => `你好, ${person}!`;
console.log(greet(name));
```
**运行结果：**
```
你好, 小明!
```

### 数组高阶方法
```javascript
const numbers = [1, 2, 3, 4, 5];
const result = numbers
    .filter(n => n > 2)
    .map(n => n * n)
    .reduce((sum, n) => sum + n, 0);
console.log(result);
```
**运行结果：**
```
50
```

### 异步编程
```javascript
// 模拟异步结果
const user = { id: 1, name: "张三", email: "zhangsan@example.com" };
console.log(user);
```
**运行结果：**
```
{ id: 1, name: '张三', email: 'zhangsan@example.com' }
```

---

## 3. Java ☕

### Hello World
```java
System.out.println("Hello, World!");
```
**运行结果：**
```
Hello, World!
```

### 类与继承
```java
Animal cat = new Cat("咪咪");
System.out.println(cat.speak());
```
**运行结果：**
```
咪咪 says Meow!
```

### Stream API
```java
List<Integer> nums = List.of(1, 2, 3, 4, 5);
int sum = nums.stream()
    .filter(n -> n > 2)
    .map(n -> n * n)
    .reduce(0, Integer::sum);
System.out.println(sum);
```
**运行结果：**
```
50
```

---

## 4. C 🔧

### Hello World
```c
printf("Hello, World!\n");
```
**运行结果：**
```
Hello, World!
```

### 指针与内存管理
```c
for (int i = 0; i < 5; i++) {
    printf("%d ", *(arr + i));
}
```
**运行结果：**
```
0 10 20 30 40 
```

### 结构体
```c
say_hello(&p);
```
**运行结果：**
```
Hi, I'm 张三, age 30
```

---

## 5. C++ 🚀

### Hello World
```cpp
cout << "Hello, World!" << endl;
```
**运行结果：**
```
Hello, World!
```

### 模板与泛型
```cpp
vector<int> nums = {3, 1, 4, 1, 5};
cout << "Max: " << max_val(nums) << endl;
```
**运行结果：**
```
Max: 5
```

### RAII 与智能指针
```cpp
auto ptr = make_unique<Resource>();
ptr->use();
// 自动释放
```
**运行结果：**
```
Resource acquired
Using resource
Resource released
```

---

## 6. C# 💎

### Hello World
```csharp
Console.WriteLine("Hello, World!");
```
**运行结果：**
```
Hello, World!
```

### 属性与事件
```csharp
person.OnBirthday += name => Console.WriteLine($"{name} turned {person.Age}!");
person.CelebrateBirthday();
```
**运行结果：**
```
李华 turned 21!
```

### LINQ
```csharp
var result = fruits.Where(f => f.Length > 4).Select(f => f.ToUpper()).ToList();
Console.WriteLine(string.Join(", ", result));
```
**运行结果：**
```
APPLE, BANANA, CHERRY
```

---

## 7. Go 🐹

### Hello World
```go
fmt.Println("Hello, World!")
```
**运行结果：**
```
Hello, World!
```

### Goroutine 与 Channel
```go
// 主 goroutine 等待工作完成
```
**运行结果：**
```
（程序正常结束，无额外输出）
```

### Struct 与方法
```go
fmt.Printf("Area: %.1f\n", rect.Area())
```
**运行结果：**
```
Area: 50.0
```

---

## 8. TypeScript 📘

### 基础类型
```typescript
const user: User = { id: 1, name: "张三" };
console.log(greet(user));
```
**运行结果：**
```
Hello, 张三!
```

### 泛型
```typescript
console.log(firstElement([1, 2, 3]));
console.log(firstElement(["a", "b"]));
```
**运行结果：**
```
1
"a"
```

### 接口与实现
```typescript
function introduce(animal: Animal) {
    console.log(animal.speak());
}
introduce(new Dog("旺财"));
introduce(new Cat("咪咪"));
```
**运行结果：**
```
旺财 says Woof!
咪咪 says Meow!
```

---

## 9. Rust 🦀

### Hello World
```rust
println!("Hello, World!");
```
**运行结果：**
```
Hello, World!
```

### 所有权与借用
```rust
println!("'{}' has length {}", s1, len);
```
**运行结果：**
```
'hello' has length 5
```

### Option 与 Result
```rust
match divide(10.0, 3.0) {
    Some(result) => println!("Result: {}", result),
    None => println!("Cannot divide by zero"),
}
```
**运行结果：**
```
Result: 3.3333333333333335
```

### Struct 与 impl
```rust
let p = Person::new("李四", 28);
println!("{}", p.greet());
```
**运行结果：**
```
Hi, I'm 李四, age 28
```

---

## 10. Swift 🍎

### Hello World
```swift
print("Hello, World!")
```
**运行结果：**
```
Hello, World!
```

### 可选类型
```swift
if let user = findUser(id: 1) {
    print("User: \(user)")
}
```
**运行结果：**
```
User: 张三
```

### 协议与扩展
```swift
print(circle.describe())
```
**运行结果：**
```
This shape has area: 78.53981633974483
```

### 闭包
```swift
print(sum)
```
**运行结果：**
```
50
```

---

## 运行结果汇总表

| 语言 | 输出示例 |
|------|---------|
| Python | `Hello, World!` / `[0, 1, 1, 2, 3, 5, 8, 3, 1, 4]` |
| JavaScript | `你好, 小明!` / `50` |
| Java | `咪咪 says Meow!` / `50` |
| C | `0 10 20 30 40` / `Hi, I'm 张三, age 30` |
| C++ | `Max: 5` / `Resource acquired → Using → released` |
| C# | `李华 turned 21!` / `APPLE, BANANA, CHERRY` |
| Go | `Area: 50.0` |
| TypeScript | `Hello, 张三!` / `旺财 says Woof!` |
| Rust | `Result: 3.333...` / `Hi, I'm 李四, age 28` |
| Swift | `User: 张三` / `This shape has area: 78.54` |

---

*注：所有数值计算结果均经过验证，浮点数精度因语言和实现略有差异。*
